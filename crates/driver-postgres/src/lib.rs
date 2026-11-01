// Postgres implementation of the driver-core traits.
mod cancel;
mod capabilities;
mod config;
mod connection;
mod dialect;
mod driver;
mod error;
mod introspect;
mod sql;
mod stream;
mod tls;
mod tx;
mod types;

pub use driver::PgDriver;
