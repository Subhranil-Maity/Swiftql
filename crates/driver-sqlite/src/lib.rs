// SQLite implementation of the driver-core traits, on rusqlite with SQLite
// compiled in (no system library needed).
mod capabilities;
mod command;
mod connection;
mod dialect;
mod driver;
mod error;
mod introspect;
mod stream;
mod types;
mod worker;

pub use driver::SqliteDriver;
