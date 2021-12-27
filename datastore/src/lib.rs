//! The datastore crate.
//!
//! This crate stores data on disk to an sqlite DB.  There are 3 primary entities:
//!
//! - The database, which takes a directory and scaffolds the sqlite db.
//! - The tablespace, which divides a database into "universes", each of which owns some subset of tables.
//! - The table, which stores rows of data.
//!
//! We go through the trouble of using one sqlite db because this lets us enable wal, provide single-file consistency,
//! and perform queries across the entire tablespace.  The last point is primarily of interest to operators: this crate
//! is designed to load data into memory, not be an ORM.
//!
//! Tables are named like `tablespace.table` as a quoted sqlite identifier, which allows for tables to use `_` (it is
//! either this or `$`, but `$` is confusing because of sql parameters). Migrations are tera templates, which contain a
//! variable for each table in the tablespace, e.g. `{{ my_table }}` instead of just `my_table`.
mod database;
mod row;
mod table;
mod tablespace;

pub use row::*;
