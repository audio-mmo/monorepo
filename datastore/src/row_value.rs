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
    I64(i64),
    Json(serde_json::Value),
    F64(f64),
    I128(i128),
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
pub(crate) struct RowValue {
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

            let cval = if v.is_null() && i.get_column_type() != &ColumnType::Json {
                if !i.is_nullable() {
                    anyhow::bail!("{}: got null value but column isn't nullable", i.get_name());
                }
                ColumnValue::Null
            } else {
                match i.get_column_type() {
                    ColumnType::I64 => ColumnValue::I64(v.as_i64().ok_or_else(|| {
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

    /// Build this row from a Rusqlite row and a given table.
    pub fn from_rusqlite_row(table: &crate::TableDescriptor, row: &rusqlite::Row) -> Result<Self> {
        use crate::ColumnType as CT;

        let mut entries = smallvec::smallvec![];

        for c in table.iter_columns() {
            let vref = row.get_ref(c.get_name())?;

            let value = if c.is_nullable() && vref == rusqlite::types::ValueRef::Null {
                ColumnValue::Null
            } else {
                match c.get_column_type() {
                    CT::I64 => ColumnValue::I64(vref.as_i64()?),
                    CT::F64 => ColumnValue::F64(vref.as_f64()?),
                    CT::String => ColumnValue::String(vref.as_str()?.to_string()),
                    CT::Json => ColumnValue::Json(serde_json::from_str(vref.as_str()?)?),
                }
            };

            entries.push(RowMapEntry {
                name: c.get_name().to_string(),
                value,
            });
        }

        Ok(RowValue {
            map: RowMap { entries },
        })
    }

    /// Build a value from this row.
    pub fn deserialize<T: serde::de::DeserializeOwned>(self) -> Result<T> {
        // We build a JSON value, then deserialize from that.  As with building row values, we can later opt to
        // implement deserializer directly if we need to.
        let mut jval = serde_json::json!({});

        let map = jval
            .as_object_mut()
            .expect("Should always succeed because we just built this value");

        for i in self.map.entries.into_iter() {
            use ColumnValue::*;

            let cval = match i.value {
                Null => serde_json::json!(null),
                F64(x) => serde_json::json!(x),
                I64(x) => serde_json::json!(x),
                String(x) => serde_json::json!(x),
                I128(x) => x.into(),
                Json(x) => x,
            };
            map.insert(i.name, cval);
        }

        Ok(serde_json::from_value(jval)?)
    }

    /// Bind the parameters of this row to a statement.  The statement is expected to have named parameters for all
    /// values in the RowValue.
    pub fn bind_params(
        &self,
        _table: &crate::TableDescriptor,
        statement: &mut rusqlite::Statement,
    ) -> Result<()> {
        use ColumnValue::*;

        for v in self.map.entries.iter() {
            let param_index = statement
                .parameter_index(&format!(":{}", v.name))?
                .ok_or_else(|| anyhow::anyhow!("Couldn't find parameter {}", &v.name))?;

            match &v.value {
                Null => statement.raw_bind_parameter(param_index, rusqlite::types::Null)?,
                I64(x) => statement.raw_bind_parameter(param_index, x)?,
                String(x) => statement.raw_bind_parameter(param_index, x)?,
                F64(x) => statement.raw_bind_parameter(param_index, x)?,
                I128(x) => statement.raw_bind_parameter(param_index, x)?,
                Json(x) => {
                    statement.raw_bind_parameter(param_index, &serde_json::to_string(&x)?)?
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[derive(
        serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq, proptest_derive::Arbitrary,
    )]
    struct TestStruct {
        pk: i64,
        name: String,
        float: f64,
        nullable_float: Option<f64>,
        nullable_string: Option<String>,
        json_map: HashMap<String, String>,
        possibly_null_json_map: Option<HashMap<String, String>>,
        jsonified_string: String,
    }

    /// We will build a reasonably complicated struct and a schema for that struct, then make sure that going through a
    /// row value and back produces the same value.
    #[test]
    fn identity() {
        use proptest::prelude::*;

        let mut schema_builder = crate::TableDescriptorBuilder::new("table".into());

        schema_builder
            .add_integer_column("pk".into(), true, false)
            .unwrap();
        schema_builder
            .add_string_column("name".into(), false, false)
            .unwrap();
        schema_builder
            .add_f64_column("float".into(), false, false)
            .unwrap();
        schema_builder
            .add_f64_column("nullable_float".into(), false, true)
            .unwrap();
        schema_builder
            .add_string_column("nullable_string".into(), false, true)
            .unwrap();
        schema_builder.add_json_column("json_map".into()).unwrap();
        schema_builder
            .add_json_column("possibly_null_json_map".into())
            .unwrap();
        schema_builder
            .add_json_column("jsonified_string".into())
            .unwrap();
        let schema = schema_builder.build().unwrap();

        proptest!(|(x: TestStruct) | {
            let rv = RowValue::new(&schema, &x).unwrap();
            let nv: TestStruct = rv.deserialize().unwrap();
            prop_assert_eq!(x, nv);
        });
    }
}
