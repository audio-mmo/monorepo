//! The datastore crate.
//!
//! This crate stores data on disk to an sqlite DB.  There are 3 primary entities:
//!
//! - The database, which takes a directory and scaffolds the sqlite db.
//! - The schema,, which divides a database into "universes", each of which owns some subset of tables.
//! - The table, which stores rows of data.
//!
//! We go through the trouble of using one sqlite db because this lets us enable wal, provide single-file consistency,
//! and perform queries across all of the data without having to worry about e.g. sqlite attached database limits.  The
//! last point is primarily of interest to operators: this crate is designed to load data into memory, not be an ORM.
//!
//! Tables are named like `schema.table` as a quoted sqlite identifier, which allows for tables to use `_` (it is either
//! this or `$`, but `$` is confusing because of sql parameters). Migrations are tera templates, which contain a
//! variable for each table in the tablespace, e.g. `{{ my_table }}` instead of just `my_table`.
//!
//! To use (see below for specifics):
//!
//! - Create a [DatabaseBuilder] with [DatabaseBuilder::New], passing it a path to the *directory* which the database
//!   will own.  There shouldn't be other files in this directory, nor should you assume that the database is only one
//!   file.  This directory must already be created.
//! - Call [DatabaseBuilder::add_schema] to add a schema.  This takes a name and a callback which receives a
//!   [SchemaBuilder].
//! - On your schema builder, call [SchemaBuilder::add_table], which takes a name and a callback which receives a
//!   [TableBuilder].
//! - On your [TableBuilder], call the `add_XXX` functions.
//! - On your [SchemaBuilder], call [SchemaBuilder::add_sql_migration] to add migrations, which will execute in the
//!   order added.
//! - Finally, do [DatabaseBuilder::build] and then [Database::open].
//! - To add data, use [Database::patch_table].
//! - To load data, use [Database::load_table].
//!
//! When building a schema, the tables should be declared in their final, post-migrated state.  This is simply metadata.
//! No implicit table creation or modification happens, not even initial table creation.  That must be done by you in
//! your migrations.
//!
//! Migrations are [tera] templates.  They receive a number of magic variables of the form `{{ tablename }}` which
//! expands to an SQL literal, e.g.:
//!
//! ```ignore
//! CREATE TABLE {{ mytable }}(...)
//! ```
//!
//! Will expand mytable appropriately to refer to the mytable in the schema the migration is being run on.
//!
//! Data is stored and loaded via serde.  The data model supports [i64], [f64], [String], and a JSON column type which
//! will store as a string containing whatever Serde serializes the column to in JSON.  Derive both [serde::Serialize]
//! and [serde::Deserialize] on a struct which will (de)serialize with field names that match the column in the declared
//! table, then you can load and patch tables.  We represent nullability with [Option], as is normal in Rust.
//!
//! This crate performs two kinds of insert, which we call a patch.  First, if the table has a primary key, we will
//! replace existing rows via `INSERT OR REPLACE`.  Otherwise, we simply do a normal insert.  The primary use for this
//! crate is to patch tables with primary keys, and the secondary use is to write various forms of log-like tables.  In
//! both these cases, this insert operation automatically does what we want (because log-like tables can't have primary
//! keys, or if they do they're already assured to be unique), and so we expose them both through the same method and
//! implicitly make the choice: see [Database::patch_table].
//!
//! The `inserter` benchmark makes a pretty good and simple exampl of how this looks, which is way less scary than these
//! docs make it sound.
#![allow(dead_code)]
pub mod database;
pub mod descriptor;
pub mod row_value;

pub use database::*;
pub use descriptor::*;
