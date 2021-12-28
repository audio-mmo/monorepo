//! Database descriptors.
//!
//! A row consists of some number of columns, each of which may have one of the following types: a 64-bit signed
//! integer, an f64, a string, or a value which should be serialized to JSON.
//!
//! A table's primary key consists of zero or more columns which are marked as primary keys.  JSON columns may not be
//! primary keys.
//!
//! All tables are grouped into a schema, which is something like Postgres's support for `schemaname.tablename`, but
//! emulated on top of sqlite.  This is what allows for multiple "things" (library, etc) to share the same sqlite file;
//! in particular, the ammo ecs and any downstream dependencies.
use std::collections::HashMap;

use anyhow::Result;

/// Types of a row's columns.
#[derive(Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub enum ColumnType {
    /// This column is a 64-bit signed integer.
    I64,
    /// This column is a string.
    String,
    /// This column is a JSON blob.
    Json,
    /// This column is an f64.
    F64,
}

/// A column in a table.
#[derive(Debug)]
pub struct ColumnDescriptor {
    name: String,
    column_type: ColumnType,
    primary_key: bool,
    nullable: bool,
}

/// Description of a table in a schema.
#[derive(Debug)]
pub struct TableDescriptor {
    name: String,
    columns: Vec<ColumnDescriptor>,
}

/// Descriptor for a migration.
///
/// For now, we require that migrations are sql.
#[derive(Debug)]
pub struct MigrationDescriptor {
    name: String,
    sql: String,
}

/// A schema, which holds a collection of tables.
#[derive(Debug)]
pub struct SchemaDescriptor {
    name: String,
    tables: HashMap<String, TableDescriptor>,
    migrations: Vec<MigrationDescriptor>,
}

#[derive(Debug)]
pub struct DatabaseDescriptor {
    path: std::path::PathBuf,
    schemas: HashMap<String, SchemaDescriptor>,
}

lazy_static::lazy_static! {
    static ref NAME_VALIDATOR: regex::Regex = {
        regex::Regex::new(r"^[a-zA-Z](\d|_|[a-zA-Z])*$").unwrap()
    };
}

impl ColumnDescriptor {
    fn new(
        name: String,
        column_type: ColumnType,
        primary_key: bool,
        nullable: bool,
    ) -> Result<Self> {
        if !NAME_VALIDATOR.is_match(&name) {
            anyhow::bail!("Column name contains invalid characters.");
        }

        if primary_key && nullable {
            anyhow::bail!("Primay key columns may not be nullable");
        }

        if primary_key && column_type == ColumnType::Json {
            anyhow::bail!("JSON columns may not be primary keys");
        }

        if nullable && column_type == ColumnType::Json {
            anyhow::bail!("JSON columns may not be NULL");
        }

        // Nothing good can ever come from primary key floats.
        if primary_key && column_type == ColumnType::F64 {
            anyhow::bail!("Floats may not be primary keys");
        }

        Ok(Self {
            name,
            column_type,
            primary_key,
            nullable,
        })
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_column_type(&self) -> &ColumnType {
        &self.column_type
    }

    pub fn is_primary_key(&self) -> bool {
        self.primary_key
    }

    pub fn is_nullable(&self) -> bool {
        self.nullable
    }
}

impl TableDescriptor {
    fn new(name: String, columns: Vec<ColumnDescriptor>) -> Result<Self> {
        use itertools::Itertools;

        {
            let mut column_refs: smallvec::SmallVec<[&str; 32]> =
                columns.iter().map(|x| x.get_name()).collect();

            column_refs.sort_unstable();
            if column_refs.iter().dedup().count() != column_refs.len() {
                anyhow::bail!("{}: Duplicate column found", name);
            }
        }

        Ok(Self { name, columns })
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn iter_columns(&self) -> impl Iterator<Item = &ColumnDescriptor> {
        self.columns.iter()
    }
}

impl MigrationDescriptor {
    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_sql(&self) -> &str {
        &self.sql
    }
}
impl SchemaDescriptor {
    fn new(
        name: String,
        tables: HashMap<String, TableDescriptor>,
        migrations: Vec<MigrationDescriptor>,
    ) -> Result<Self> {
        if !NAME_VALIDATOR.is_match(&name) {
            anyhow::bail!("Invalid schema name {}", name);
        }

        Ok(SchemaDescriptor {
            name,
            tables,
            migrations,
        })
    }

    pub fn iter_tables(&self) -> impl Iterator<Item = &TableDescriptor> {
        self.tables.values()
    }

    pub fn get_name(&self) -> &str {
        self.name.as_str()
    }

    pub fn iter_migrations(&self) -> impl Iterator<Item = &MigrationDescriptor> {
        self.migrations.iter()
    }
}

impl DatabaseDescriptor {
    fn new(path: std::path::PathBuf, schemas: HashMap<String, SchemaDescriptor>) -> Result<Self> {
        Ok(DatabaseDescriptor { path, schemas })
    }

    pub fn iter_schemas(&self) -> impl Iterator<Item = &SchemaDescriptor> {
        self.schemas.values()
    }

    pub fn get_path(&self) -> &std::path::Path {
        self.path.as_path()
    }

    /// get a reference to a  given table in the database from the schema/name pair, returning an error if this wasn't possible.

    pub fn get_table_from_params(&self, schema: &str, table: &str) -> Result<&TableDescriptor> {
        self.schemas
            .get(schema)
            .ok_or_else(|| anyhow::anyhow!("Schema {} not found", schema))?
            .tables
            .get(table)
            .ok_or_else(|| anyhow::anyhow!("Table {} not found in schema {}", table, schema))
    }
}

/// A builder for tables.
#[derive(Debug)]
pub struct TableDescriptorBuilder {
    name: String,
    columns: Vec<ColumnDescriptor>,
}

impl TableDescriptorBuilder {
    pub fn new(name: String) -> Self {
        Self {
            name,
            columns: vec![],
        }
    }

    fn check_name(&self, name: &str) -> Result<()> {
        if self.columns.iter().map(|x| x.get_name()).any(|x| x == name) {
            anyhow::bail!("Duplicate column names not allowed");
        }
        Ok(())
    }

    pub fn add_json_column(&mut self, name: String) -> Result<()> {
        self.check_name(&name)?;
        // JSON isn't deterministic enough to be a primary key.  It's also never null, since we can just store JSON null
        // in the column.
        self.columns
            .push(ColumnDescriptor::new(name, ColumnType::Json, false, false)?);
        Ok(())
    }

    pub fn add_integer_column(
        &mut self,
        name: String,
        primary_key: bool,
        nullable: bool,
    ) -> Result<()> {
        self.check_name(&name)?;
        self.columns.push(ColumnDescriptor::new(
            name,
            ColumnType::I64,
            primary_key,
            nullable,
        )?);
        Ok(())
    }

    pub fn add_string_column(
        &mut self,
        name: String,
        primary_key: bool,
        nullable: bool,
    ) -> Result<()> {
        self.check_name(&name)?;
        self.columns.push(ColumnDescriptor::new(
            name,
            ColumnType::String,
            primary_key,
            nullable,
        )?);
        Ok(())
    }

    pub fn add_f64_column(
        &mut self,
        name: String,
        primary_key: bool,
        nullable: bool,
    ) -> Result<()> {
        self.check_name(&name)?;
        self.columns.push(ColumnDescriptor::new(
            name,
            ColumnType::F64,
            primary_key,
            nullable,
        )?);
        Ok(())
    }

    pub fn build(self) -> Result<TableDescriptor> {
        TableDescriptor::new(self.name, self.columns)
    }
}

/// Builder for schemas.
#[derive(Debug)]
pub struct SchemaDescriptorBuilder {
    name: String,
    tables: HashMap<String, TableDescriptor>,
    migrations: Vec<MigrationDescriptor>,
}

impl SchemaDescriptorBuilder {
    pub fn new(name: String) -> Self {
        SchemaDescriptorBuilder {
            name,
            tables: Default::default(),
            migrations: vec![],
        }
    }

    /// Add a table.  Takes a closure which will be passed a reference to a builder, which we build for you once the
    /// closure returns.
    pub fn add_table<F>(&mut self, name: String, table_builder: F) -> Result<()>
    where
        F: for<'a> FnOnce(&'a mut TableDescriptorBuilder) -> Result<()>,
    {
        if self.tables.contains_key(&name) {
            anyhow::bail!("Schema {}: Duplicate table name {}", self.name, name);
        }

        let mut builder = TableDescriptorBuilder::new(name.clone());
        table_builder(&mut builder)?;
        let table = builder.build()?;
        self.tables.insert(name, table);

        Ok(())
    }

    /// Add a migration.
    ///
    /// Migrations will be run on this schema in the order they are added to the builder, but do not run in any
    /// particular order with respect to other schemas.
    pub fn add_sql_migration(&mut self, name: String, sql: String) -> Result<()> {
        if name.is_empty() {
            anyhow::bail!("Migration names must be non-empty")
        };
        if self.migrations.iter().any(|x| x.name == name) {
            anyhow::bail!("Schema {}: duplicate migration named {}", self.name, name);
        }

        // We allow empty sql.  Such a migration can run, and is useful if we ever have to yank data.
        self.migrations.push(MigrationDescriptor { name, sql });
        Ok(())
    }

    pub fn build(self) -> Result<SchemaDescriptor> {
        SchemaDescriptor::new(self.name, self.tables, self.migrations)
    }
}

/// Builder for databases.
#[derive(Debug)]
pub struct DatabaseDescriptorBuilder {
    path: std::path::PathBuf,
    schemas: HashMap<String, SchemaDescriptor>,
}

impl DatabaseDescriptorBuilder {
    pub fn new(path: std::path::PathBuf) -> Self {
        DatabaseDescriptorBuilder {
            path,
            schemas: Default::default(),
        }
    }

    /// Add a schema.  Takes a closure which will be passed a reference to a builder, which we build for you once the
    /// closure returns.
    pub fn add_schema<F>(&mut self, name: String, schema_builder: F) -> Result<()>
    where
        F: for<'a> FnOnce(&'a mut SchemaDescriptorBuilder) -> Result<()>,
    {
        if self.schemas.contains_key(&name) {
            anyhow::bail!(
                "Database {}: Duplicate schema name {}",
                self.path.display(),
                name
            );
        }

        let mut builder = SchemaDescriptorBuilder::new(name.clone());
        schema_builder(&mut builder)?;
        let schema = builder.build()?;
        self.schemas.insert(name, schema);

        Ok(())
    }

    pub fn build(self) -> Result<DatabaseDescriptor> {
        DatabaseDescriptor::new(self.path, self.schemas)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Check that the column validation only allows valid things through.
    #[test]
    fn column_validation() {
        use ColumnType::*;

        for (name, ctype, primary_key, is_nullable, is_good) in [
            ("test", I64, true, false, true),
            ("1test", String, false, false, false),
            ("a b", String, false, false, false),
            ("good_json", Json, false, false, true),
            ("no_null_json", Json, false, true, false),
            ("no_pk_json", Json, true, false, false),
            ("no_null_pk", I64, true, true, false),
            ("numbers_at_end_work1", I64, false, false, true),
            ("no_float_pk", F64, true, false, false),
        ] {
            assert!(
                ColumnDescriptor::new(name.to_string(), ctype, primary_key, is_nullable).is_ok()
                    == is_good,
                "{}",
                name
            );
        }
    }
}
