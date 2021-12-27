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

/// Description of a table.
#[derive(Debug)]
pub struct TableDescriptor {
    name: String,
    columns: Vec<ColumnDescriptor>,
}

impl ColumnDescriptor {
    pub fn new(
        name: String,
        column_type: ColumnType,
        primary_key: bool,
        nullable: bool,
    ) -> Result<Self> {
        lazy_static::lazy_static! {
            static ref NAME_VALIDATOR: regex::Regex = {
                regex::Regex::new(r"^[a-zA-Z](\d|_|[a-zA-Z])*$").unwrap()
            };
        };

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
    fn new(name: String, columns: Vec<ColumnDescriptor>) -> Self {
        Self { name, columns }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn iter_columns(&self) -> impl Iterator<Item = &ColumnDescriptor> {
        self.columns.iter()
    }
}

/// A helper to build tables.
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
        Ok(TableDescriptor::new(self.name, self.columns))
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
