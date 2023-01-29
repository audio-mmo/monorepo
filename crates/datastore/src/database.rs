//! The database wraps a rusqlite connection and provides the ability to work with a schema.
use std::collections::HashMap;

use anyhow::Result;
use log::*;

use crate::DatabaseDescriptor;

struct DatabaseState {
    /// Maps table name to prebuilt insert statements for the table.
    insert_statements: HashMap<String, String>,

    /// Maps table name to statements to load from the table.
    load_statements: HashMap<String, String>,

    descriptor: DatabaseDescriptor,
}

pub struct Database {
    conn: rusqlite::Connection,
    state: DatabaseState,
}

/// A transaction like that from rusqlite: drop rolls back, calling commit commits.
pub struct Transaction<'a> {
    state: &'a DatabaseState,
    transaction: rusqlite::Transaction<'a>,
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
    {{ columns | join(sep=", ") }}
) values (
    {%- for c in columns -%}
    :{{ c}}{% if not loop.last %},{% endif -%}
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
SELECT {% for c in columns %}{{ c}} as {{ c }}{% if not loop.last %}, {% endif %}{%endfor%}
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
            // First we need to know if we already ran it.
            let had_migration = transaction
                .prepare("SELECT * FROM migrations where schema = ? AND name = ?")?
                .exists(rusqlite::params![schema.get_name(), mig.get_name()])?;
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

        let path = descriptor.get_path().join("database.sqlite");
        info!("Opening database at {}", path.display());
        info!(
            "{} has the following tables: {}",
            descriptor.get_path().display(),
            iter_all_tables(&descriptor)
                .map(|x| build_table_ident(x.0, x.1.get_name()))
                .join(", ")
        );
        let conn = rusqlite::Connection::open(&path)?;
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
            state: DatabaseState {
                load_statements,
                insert_statements,
                descriptor,
            },
            conn,
        })
    }

    pub fn transaction(&mut self) -> Result<Transaction> {
        Ok(Transaction {
            state: &self.state,
            transaction: self.conn.transaction()?,
        })
    }
}

impl<'a> Transaction<'a> {
    /// Load a table, calling the user-specified function with each returned row.
    ///
    /// This function can fail in the middle, but will always pass valid objects to your callback.  SO e.g. if you see
    /// 500 and then a failure, there might have been 2000.
    pub fn load_table<T: serde::de::DeserializeOwned>(
        &self,
        schema: &str,
        table: &str,
        mut callback: impl FnMut(T) -> Result<()>,
    ) -> Result<()> {
        let table_desc = self.state.descriptor.get_table_from_params(schema, table)?;
        let query_text = self
            .state
            .load_statements
            .get(&build_table_ident(schema, table))
            .expect("If we have a valid table, we should have an insert statement for it.");
        let mut statement = self.transaction.prepare_cached(query_text)?;

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
    pub fn patch_table<T: serde::Serialize>(
        &mut self,
        schema: &str,
        table: &str,
        values: &[T],
    ) -> Result<()> {
        let table_desc = self.state.descriptor.get_table_from_params(schema, table)?;
        let query_text = self
            .state
            .insert_statements
            .get(&build_table_ident(schema, table))
            .expect("We can't have a table without a statement");
        let mut statement = self.transaction.prepare_cached(query_text)?;

        // The statement can be reused: row values always bind all parameters.
        for v in values.iter() {
            // First, from the external value to a row value:
            let rv = crate::row_value::RowValue::new(table_desc, v)?;
            // Then bind.
            rv.bind_params(table_desc, &mut statement)?;
            // And then run it.
            statement.raw_execute()?;
        }

        Ok(())
    }

    /// Truncate the specified table.
    ///
    /// Deletes all contents of the table.
    pub fn truncate_table(&self, schema: &str, table: &str) -> Result<()> {
        // Make sure it actually exists.
        self.state.descriptor.get_table_from_params(schema, table)?;
        let table_name = build_table_ident(schema, table);
        self.transaction
            .execute(&format!("delete from {}", table_name), [])?;
        Ok(())
    }

    /// Delete the contents of all tables.  Primarily useful for testing.
    pub fn truncate_all_tables(&mut self) -> Result<()> {
        for (schema, table) in iter_all_tables(&self.state.descriptor) {
            self.transaction.execute(
                &format!(
                    "delete from {}",
                    build_table_ident(schema, table.get_name())
                ),
                [],
            )?;
        }

        Ok(())
    }

    pub fn commit(self) -> Result<()> {
        Ok(self.transaction.commit()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::HashMap;

    /// Goes with the test schema, below.
    #[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
    struct TestRow {
        primary_key: i64,
        string_col: String,
        f64_col: Option<f64>,
        // Anything that goes to JSON is fine, but let's get coverage on the NULL.
        json: Option<HashMap<String, String>>,
    }

    /// Build a simple test schema, with a couple tables (intentionally of the same name).
    fn build_test_descriptor(path: &std::path::Path) -> Result<DatabaseDescriptor> {
        let mut desc_builder = crate::DatabaseDescriptorBuilder::new(path.to_path_buf());

        for schema in ["schema1", "schema2"] {
            desc_builder.add_schema(schema.to_string(), |b| {
                for table in ["t1", "t2"] {
                    b.add_table(table.to_string(), |tb| {
                        tb.add_integer_column("primary_key".into(), true, false)?;
                        tb.add_string_column("string_col".into(), false, false)?;
                        tb.add_f64_column("f64_col".into(), false, true)?;
                        tb.add_json_column("json".into())?;
                        Ok(())
                    })?;

                    let braced_table = format!("{{{{ {table} }}}}");
                    // We'll build the table in 2 migrations: one to create some of the columns, and one that uses alter
                    // column for the rest.
                    let m1 = format!(
                        r#"
                        CREATE TABLE {braced_table} (
                            primary_key INTEGER PRIMARY KEY,
                            string_col TEXT NOT NULL
                        );
                    "#
                    );
                    let m2 = format!(
                        r#"
                        ALTER TABLE {braced_table} ADD COLUMN f64_col REAL;
                        ALTER TABLE {braced_table} ADD COLUMN json TEXT NOT NULL;
                    "#
                    );
                    b.add_sql_migration(format!("m1.{}", table), m1)?;
                    b.add_sql_migration(format!("m2.{}", table), m2)?;
                }

                Ok(())
            })?;
        }

        desc_builder.build()
    }

    #[test]
    fn opens() {
        let tdir = tempfile::TempDir::new().unwrap();
        let desc = build_test_descriptor(tdir.path()).unwrap();
        Database::open(desc).expect("Database should open");
    }

    /// Will detect if we ran migrations twice.
    #[test]
    fn opens_twice() {
        let tdir = tempfile::TempDir::new().unwrap();
        let desc = build_test_descriptor(tdir.path()).unwrap();
        Database::open(desc).expect("Database should open");
        let desc = build_test_descriptor(tdir.path()).unwrap();
        Database::open(desc).expect("Database should open");
    }

    /// Test all the I/O pieces.  It's easiest to just do this all at once, rather than to try to write separate tests.
    ///
    /// Other ammo crates will eventually offer additional coverage indirectly.
    #[test]
    fn test_table_manipulation() {
        let tdir = tempfile::TempDir::new().unwrap();
        let desc = build_test_descriptor(tdir.path()).unwrap();
        let mut db = Database::open(desc).expect("Database should open");

        let mut expected_rows: HashMap<String, HashMap<String, Vec<TestRow>>> = Default::default();

        for s in ["schema1", "schema2"] {
            expected_rows.insert(s.to_string(), Default::default());
            let schema_map = expected_rows.get_mut(s).unwrap();

            for t in ["t1", "t2"] {
                let mut json_map = HashMap::new();
                json_map.insert("a".into(), "b".into());
                json_map.insert("c".into(), format!("{}.{}", s, t));

                // Let's make a set of rows:
                let mut rows = (0i64..1)
                    .map(|x| TestRow {
                        primary_key: x,
                        f64_col: Some(x as f64),
                        json: Some(json_map.clone()),
                        string_col: format!("string.{}.{}.{}", s, t, x),
                    })
                    .collect::<Vec<_>>();
                // let's get coverage on the NULL:
                rows.push(TestRow {
                    primary_key: 11,
                    f64_col: None,
                    json: None,
                    string_col: "foo".into(),
                });

                let mut transaction = db.transaction().expect("Should make a transaction");
                transaction
                    .patch_table(s, t, &rows[..])
                    .expect("Insert should succeed");
                transaction.commit().expect("Should commit");
                schema_map.insert(t.to_string(), rows);
            }
        }

        // At this point, we have inserted a bunch of rows. Let's reload them and make sure they're present.
        for (schema, tables) in expected_rows.iter() {
            for (table, expected_rows) in tables.iter() {
                let mut rows = vec![];
                let transaction = db.transaction().unwrap();
                transaction
                    .load_table(schema, table, |x: TestRow| {
                        rows.push(x);
                        Ok(())
                    })
                    .expect("Load should succeed");
                assert_eq!(&rows, expected_rows);
            }
        }

        // Now, let's patch all our rows and check that updating works.
        for tables in expected_rows.values_mut() {
            for rows in tables.values_mut() {
                rows.iter_mut().enumerate().for_each(|(i, x)| {
                    x.string_col = format!("patched{}", i);
                });
            }
        }

        // And re-insert, but this time all as part of the same transaction:
        let mut transaction = db.transaction().unwrap();
        for (schema, tables) in expected_rows.iter() {
            for (table, rows) in tables.iter() {
                transaction
                    .patch_table(schema, table, &rows[..])
                    .expect("Should patch");
            }
        }

        transaction.commit().unwrap();

        // Check after the patch.
        for (schema, tables) in expected_rows.iter() {
            for (table, expected_rows) in tables.iter() {
                let mut rows = vec![];
                db.transaction()
                    .unwrap()
                    .load_table(schema, table, |x: TestRow| {
                        rows.push(x);
                        Ok(())
                    })
                    .expect("Load should succeed");
                assert_eq!(&rows, expected_rows);
            }
        }

        // Now reset the database, and check that it's empty.
        let mut transaction = db.transaction().unwrap();
        transaction.truncate_all_tables().expect("Should delete");
        transaction.commit().unwrap();
        for schema in ["schema1", "schema2"] {
            for table in ["t1", "t2"] {
                let mut rows = vec![];
                db.transaction()
                    .unwrap()
                    .load_table(schema, table, |x: TestRow| {
                        rows.push(x);
                        Ok(())
                    })
                    .expect("Should load");
                assert!(rows.is_empty());
            }
        }
    }
}
