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
INSERT INTO {{ table}}(
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
        conn: rusqlite::Connection,
        descriptor: DatabaseDescriptor,
    ) -> Result<Self> {
        let load_statements = build_load_statements(&descriptor)?;
        let insert_statements = build_insert_statements(&descriptor)?;
        conn.execute_batch(INITIAL_SQL)?;
        Ok(Database {
            conn,
            load_statements,
            insert_statements,
            descriptor,
        })
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
}
