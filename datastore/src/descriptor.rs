//! Rows.
//!
//! A row consists of some number of columns, each of which may have one of 3 types: a 64-bit signed integer, a string,
//! or a value which should be serialized to JSON.
//!
//! A table's primary key consists of zero or more columns which are marked as primary keys.  JSON columns may not be
//! primary keys.
use anyhow::Result;

/// Types of a row's columns.
#[derive(Debug)]
pub enum ColumnType {
    /// This column is a 64-bit signed integer.
    Integer,
    /// This column is a string.
    String,
    /// This column is a JSON blob.
    Json,
}

/// A column in a table.
#[derive(Debug)]
pub struct ColumnDescriptor {
    name: String,
    column_type: ColumnType,
    primary_key: bool,
}

/// Description of a table.
#[derive(Debug)]
pub struct TableDescriptor {
    name: String,
    columns: Vec<ColumnDescriptor>,
}

impl ColumnDescriptor {
    pub fn new(name: String, column_type: ColumnType, primary_key: bool) -> Self {
        Self {
            name,
            column_type,
            primary_key,
        }
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
}

impl TableDescriptor {
    fn new(name: String, columns: Vec<ColumnDescriptor>) -> Self {
        Self { name, columns }
    }
}

/// A helper to build tables.
pub struct TableBuilder {
    name: String,
    columns: Vec<ColumnDescriptor>,
}

impl TableBuilder {
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
        self.columns
            .push(ColumnDescriptor::new(name, ColumnType::Json, false));
        Ok(())
    }

    pub fn add_integer_column(&mut self, name: String, primary_key: bool) -> Result<()> {
        self.check_name(&name)?;
        self.columns.push(ColumnDescriptor::new(
            name,
            ColumnType::Integer,
            primary_key,
        ));
        Ok(())
    }

    pub fn add_string_column(&mut self, name: String, primary_key: bool) -> Result<()> {
        self.check_name(&name)?;
        self.columns
            .push(ColumnDescriptor::new(name, ColumnType::String, primary_key));
        Ok(())
    }

    pub fn build(self) -> Result<TableDescriptor> {
        Ok(TableDescriptor::new(self.name, self.columns))
    }
}
