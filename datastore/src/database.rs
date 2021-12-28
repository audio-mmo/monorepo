//! The database wraps a rusqlite connection and provides the ability to work with a schema.
use std::collections::HashMap;

use anyhow::Result;
use log::*;

use crate::DatabaseDescriptor;

pub struct Database {
    conn: rusqlite::Connection,

    /// Maps table name to prebuilt insert statements for the table.
    insert_statements: HashMap<String, String>,

    /// Maps table name to statements to load from the table.
    load_statements: HashMap<String, String>,

    descriptor: DatabaseDescriptor,
}

/// SQL that we run as part of opening a connection.
///
/// - Sets up WAL.
/// - Enables the busy timeout
/// - Makes sure the WAL file is truncated because they can grow quite large under some obscure conditions that are way
///   too involved to stick in anything less than a full blog post.
/// - Enables foreign key enforcement (though we don't expect foreign keys to be used).
/// - Raises the default cache size because the one sqlite sets up for us is only a couple megabytes since they have to
///   make their default swork with e.g. phones.
const INITIAL_SQL: &str = r#"
PRAGMA busy_timeout = 1000;
PRAGMA cache_size = -100000;
PRAGMA foreign_keys = 1;
pragma journal_mode = WAL;
PRAGMA wal_autocheckpoint = 10000;
PRAGMA wal_checkpoint(full);
"#;

/// Returns an iterator `(schemaname, table_descriptor)`.
fn iter_all_tables(
    descriptor: &DatabaseDescriptor,
) -> impl Iterator<Item = (&str, &crate::TableDescriptor)> {
    descriptor
        .iter_schemas()
        .flat_map(|s| s.iter_tables().map(|t| (s.get_name(), t)))
}

/// Build the `schema.table` identifier.
fn build_table_ident(schema: &str, table: &str) -> String {
    format!("`{}.{}`", schema, table)
}

const INSERT_TEMPLATE: &str = r#"
INSERT {% if has_pk %}OR REPLACE{% endif %} INTO {{ table}}(
    {%- for c in columns -%}
        {{- c }},
    {%- endfor -%}
) values (
    {%- for c in columns -%}
        :{{ c }}
    {%- endfor -%}
)
"#;

/// Build the insert statements for all schemas and tables in the database.
fn build_insert_statements(descriptor: &DatabaseDescriptor) -> Result<HashMap<String, String>> {
    let mut ret: HashMap<String, String> = Default::default();

    for (schema, table) in iter_all_tables(descriptor) {
        let table_ident = build_table_ident(schema, table.get_name());
        let mut context = tera::Context::new();
        context.insert("table", &table_ident);
        context.insert(
            "columns",
            &table
                .iter_columns()
                .map(|x| x.get_name())
                .collect::<Vec<_>>(),
        );
        context.insert("has_pk", &table.iter_columns().any(|x| x.is_primary_key()));

        let stmt = tera::Tera::one_off(INSERT_TEMPLATE, &context, false)?;
        debug!("Insert statement for {}: {}", table_ident, stmt);
        ret.insert(table_ident, stmt);
    }

    Ok(ret)
}

const LOAD_TEMPLATE: &str = r#"
SELECT ({% for c in columns %}{{ c}} as {{ c }}{%endfor%})
FROM {{ table }}
"#;

/// Build the load statements.
fn build_load_statements(descriptor: &DatabaseDescriptor) -> Result<HashMap<String, String>> {
    let mut ret: HashMap<String, String> = Default::default();

    for (schema, table) in iter_all_tables(descriptor) {
        let table_ident = build_table_ident(schema, table.get_name());
        let mut context = tera::Context::new();
        context.insert("table", &table_ident);
        context.insert(
            "columns",
            &table
                .iter_columns()
                .map(|x| x.get_name())
                .collect::<Vec<_>>(),
        );

        let stmt = tera::Tera::one_off(LOAD_TEMPLATE, &context, false)?;
        debug!("Load statement for {}: {}", table_ident, stmt);
        ret.insert(table_ident, stmt);
    }

    Ok(ret)
}

/// Run the migrations for a given database, creating the initial migrations infrastructure if necessary.
///
/// Note that the initial migrations table is, in effect, the only thing we can't migrate without a lot of work.
fn run_migrations(conn: &mut rusqlite::Connection, descriptor: &DatabaseDescriptor) -> Result<()> {
    let transaction = conn.transaction()?;

    // First, we create our migrations table.
    transaction.execute(r#"CREATE TABLE IF NOT EXISTS migrations (
        -- Incrementing, unique id for the migration.
        id INTEGER,
        -- Schema the migration was for.
        schema TEXT NOT NULL,
        -- Name of the migration.
        name TEXT NOT NULL,
        -- The specific sql run for this migration after template rendering, which can be useful for debugging.
        sql TEXT NOT NULL,
        -- Unix timestamp as real seconds
        ran_at REAL,
        -- Duration taken as real seconds.
        duration REAL NOT NULL
    )"#, [])?;

    for schema in descriptor.iter_schemas() {
        let mut ctx = tera::Context::new();

        // Add all our table identifiers to the template, so that a migration may `{{ tablename }}`.
        for table in schema.iter_tables() {
            ctx.insert(
                table.get_name(),
                &build_table_ident(schema.get_name(), table.get_name()),
            );
        }

        for mig in schema.iter_migrations() {
            use rusqlite::OptionalExtension;

            // First we need to know if we already ran it.
            let had_migration = transaction
                .execute(
                    "SELECT id FROM migrations where schema = ?AND name = ?",
                    rusqlite::params![schema.get_name(), mig.get_name()],
                )
                .optional()?
                .is_some();
            if had_migration {
                continue;
            }

            let ran_at = (std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)?)
            .as_secs_f64();

            let start_time = std::time::Instant::now();
            let statements = tera::Tera::one_off(mig.get_sql(), &ctx, false)?;
            transaction.execute_batch(&statements)?;
            let end_time = std::time::Instant::now();
            let duration = (end_time - start_time).as_secs_f64();

            // Now record that we ran it.
            transaction.execute(
                "INSERT INTO migrations(schema, name, sql, ran_at, duration) VALUES(?, ?, ?, ?, ?)",
                rusqlite::params![
                    schema.get_name(),
                    mig.get_name(),
                    statements.as_str(),
                    ran_at,
                    duration
                ],
            )?;
        }
    }

    // Let's make sure that no one has played around with foreign keys.
    transaction.execute("PRAGMA FOREIGN_KEY_CHECK", [])?;
    transaction.execute("PRAGMA foreign_keys = 1", [])?;

    transaction.commit()?;
    Ok(())
}

impl Database {
    pub fn open(descriptor: DatabaseDescriptor) -> Result<Self> {
        use itertools::Itertools;

        info!("Opening database at {}", descriptor.get_path().display());
        info!(
            "{} has the following tables: {}",
            descriptor.get_path().display(),
            iter_all_tables(&descriptor)
                .map(|x| build_table_ident(x.0, x.1.get_name()))
                .join(", ")
        );
        let conn = rusqlite::Connection::open(descriptor.get_path())?;
        Database::with_connection(conn, descriptor)
    }

    /// Build a database from an already-existing connection.
    ///
    /// This should be used for testing only.
    pub fn with_connection(
        mut conn: rusqlite::Connection,
        descriptor: DatabaseDescriptor,
    ) -> Result<Self> {
        let load_statements = build_load_statements(&descriptor)?;
        let insert_statements = build_insert_statements(&descriptor)?;
        conn.execute_batch(INITIAL_SQL)?;
        run_migrations(&mut conn, &descriptor)?;
        Ok(Database {
            conn,
            load_statements,
            insert_statements,
            descriptor,
        })
    }

    /// Load a table, calling the user-specified function with each returned row.
    ///
    /// This function can fail in the middle, but will always pass valid objects to your callback.  SO e.g. if you see
    /// 500 and then a failure, there might have been 2000.
    pub fn load_table<T: serde::de::DeserializeOwned>(
        &self,
        schema: &str,
        table: &str,
        callback: impl Fn(T) -> Result<()>,
    ) -> Result<()> {
        let table_desc = self.descriptor.get_table_from_params(schema, table)?;
        let query_text = self
            .load_statements
            .get(&build_table_ident(schema, table))
            .expect("If we have a valid table, we should have an insert statement for it.");
        let mut statement = self.conn.prepare_cached(query_text)?;

        let mut rows = statement.query([])?;
        while let Some(r) = rows.next()? {
            let rv = crate::row_value::RowValue::from_rusqlite_row(table_desc, r)?;
            callback(rv.deserialize()?)?;
        }

        Ok(())
    }

    /// Patch a table.  This means:
    ///
    /// - For any table without a primary key, just insert the rows; or
    /// - If the table has a primary key, insert or replace.
    ///
    /// This is designed to allow for partial rewrites of large tables with primary keys.
    fn patch_table<T: serde::Serialize>(
        &mut self,
        schema: &str,
        table: &str,
        values: &[T],
    ) -> Result<()> {
        let transaction = self.conn.transaction()?;
        let table_desc = self.descriptor.get_table_from_params(schema, table)?;
        let query_text = self
            .insert_statements
            .get(&build_table_ident(schema, table))
            .expect("We can't have a table without a statement");
        let mut statement = transaction.prepare_cached(query_text)?;

        // The statement can be reused: row values always bind all parameters.
        for v in values.iter() {
            // First, from the external value to a row value:
            let rv = crate::row_value::RowValue::new(table_desc, v)?;
            // Then bind.
            rv.bind_params(table_desc, &mut statement)?;
            // And then run it.
            statement.execute([])?;
        }

        std::mem::drop(statement);
        transaction.commit()?;
        Ok(())
    }

    /// Truncate the specified table.
    ///
    /// Deletes all contents of the table.
    pub fn truncate_table(&self, schema: &str, table: &str) -> Result<()> {
        // Make sure it actually exists.
        self.descriptor.get_table_from_params(schema, table)?;
        let table_name = build_table_ident(schema, table);
        self.conn
            .execute(&format!("delete from {}", table_name), [])?;
        Ok(())
    }

    /// Delete the contents of all tables.  Primarily useful for testing.
    pub fn truncate_all_tables(&mut self) -> Result<()> {
        let transaction = self.conn.transaction()?;

        for (schema, table) in iter_all_tables(&self.descriptor) {
            transaction.execute(
                &format!(
                    "delete from {}",
                    build_table_ident(schema, table.get_name())
                ),
                [],
            )?;
        }

        transaction.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a simple test schema, with a couple tables (intentionally of the same name).
    fn build_test_descriptor(path: &std::path::Path) -> Result<DatabaseDescriptor> {
        let mut desc_builder = crate::DatabaseDescriptorBuilder::new(path.to_path_buf());

        for schema in ["schema1", "schema2"] {
            desc_builder.add_schema(schema.to_string(), |b| {
                for table in ["t1", "t2"] {
                    b.add_table(table.to_string(), |tb| {
                        tb.add_integer_column("primary_key".into(), true, false)?;
                        tb.add_string_column("string".into(), false, false)?;
                        tb.add_f64_column("f64".into(), false, true)?;
                        tb.add_json_column("json".into())?;
                        Ok(())
                    })?;

                    // We'll build the table in 2 migrations: one to create some of the columns, and one that uses altar
                    // column for the rest.
                    let m1 = format!(
                        r#"
                        CREATE TABLE {table} (
                            primary_key INTEGER PRIMARY KEY,
                            string TEXT NOT NULL
                        );
                    "#,
                        table = format!("{{{{ {} }}}}", table),
                    );
                    let m2 = format!(
                        r#"
                        ALTAR TABLE {table} ADD COLUMN f64 REAL;
                        ALTAR TABLE {table} ADD COLUMN json TEXT NOT NULL;
                    "#,
                        table = format!("{{{{ {} }}}}", table),
                    );
                    b.add_sql_migration("m1".into(), m1)?;
                    b.add_sql_migration("m2".into(), m2)?;
                }

                Ok(())
            })?;
        }

        desc_builder.build()
    }

    #[test]
    fn opens() {
        let tdir = tempfile::TempDir::new().unwrap();
        let mut path = tdir.path().to_path_buf();
        path.push("database.db");
        let desc = build_test_descriptor(&path).unwrap();
        Database::open(desc).expect("Database should open");
    }

    /// Will detect if we ran migrations twice.
    #[test]
    fn opens_twice() {
        let tdir = tempfile::TempDir::new().unwrap();
        let mut path = tdir.path().to_path_buf();
        path.push("database.db");
        let desc = build_test_descriptor(&path).unwrap();
        Database::open(desc).expect("Database should open");
        let desc = build_test_descriptor(&path).unwrap();
        Database::open(desc).expect("Database should open");
    }
}
