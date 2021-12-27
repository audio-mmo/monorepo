//! An opaque value of a row, used for storing and loading data.
//!
//! get one with [RowValue::new_with_descriptor], which takes a table descriptor and a serde-serializable value,.
use anyhow::Result;
use smallvec::SmallVec;

use crate::descriptor::{ColumnType, TableDescriptor};

#[derive(Debug)]
enum ColumnValue {
    Null,
    String(String),
    Integer(i64),
    Json(serde_json::Value),
    F64(f64),
}

/// We anticipate that we will be building and destroying tons and tons of rows, so instead of using a hashmap etc. we
/// use a `SmallVec` map when we can.  These are the entries in that map.
#[derive(Debug)]
struct RowMapEntry {
    name: String,
    value: ColumnValue,
}

#[derive(Debug, Default)]
struct RowMap {
    entries: SmallVec<[RowMapEntry; 32]>,
}

#[derive(Debug)]
struct RowValue {
    map: RowMap,
}

impl RowValue {
    /// Make a row for the specified table.
    pub fn new(descriptor: &TableDescriptor, value: &impl serde::Serialize) -> Result<RowValue> {
        // For now, we go through serde_json because it is a convenient way to quickly get a parseable enum.  If this
        // proves to be too slow, we can instead implement a custom serializer.
        let mut json = serde_json::to_value(value)?;
        let mut map: RowMap = Default::default();

        for i in descriptor.iter_columns() {
            let v = json.get_mut(i.get_name()).ok_or_else(|| {
                anyhow::anyhow!(
                    "Input struct doesn't have field for column {}",
                    i.get_name()
                )
            })?;

            let cval = if v.is_null() {
                if !i.is_nullable() {
                    anyhow::bail!("{}: got null value but column isn't nullable", i.get_name());
                }
                ColumnValue::Null
            } else {
                match i.get_column_type() {
                    ColumnType::Integer => ColumnValue::Integer(v.as_i64().ok_or_else(|| {
                        anyhow::anyhow!("{}: integer isn't representable as i64", i.get_name())
                    })?),
                    ColumnType::String => ColumnValue::String(
                        v.as_str()
                            .ok_or_else(|| anyhow::anyhow!("{}: should be a string", i.get_name()))?
                            .to_string(),
                    ),
                    ColumnType::Json => ColumnValue::Json(v.take()),
                    ColumnType::F64 => ColumnValue::F64(
                        v.as_f64()
                            .ok_or_else(|| anyhow::anyhow!("{}: isn't an f64", i.get_name()))?,
                    ),
                }
            };

            map.entries.push(RowMapEntry {
                name: i.get_name().to_string(),
                value: cval,
            });
        }

        Ok(RowValue { map })
    }
}
