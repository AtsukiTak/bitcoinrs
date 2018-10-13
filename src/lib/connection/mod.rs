mod error;

pub mod socket;
mod connection;

pub use self::connection::Connection;
pub use self::error::ConnectionError;
