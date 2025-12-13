//! Error types for XDR serialization/deserialization.

use std::fmt;

/// Result type for XDR operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur during XDR serialization/deserialization.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Custom error message from serde.
    #[error("{0}")]
    Message(String),

    /// Unexpected end of input.
    #[error("unexpected end of input")]
    Eof,

    /// Invalid boolean value (must be 0 or 1).
    #[error("invalid boolean value: {0}")]
    InvalidBool(u32),

    /// Invalid enum discriminant.
    #[error("invalid enum discriminant: {0}")]
    InvalidEnumDiscriminant(i32),

    /// String is not valid UTF-8.
    #[error("invalid UTF-8 string")]
    InvalidUtf8,

    /// String exceeds maximum length.
    #[error("string length {0} exceeds maximum {1}")]
    StringTooLong(usize, usize),

    /// Array exceeds maximum length.
    #[error("array length {0} exceeds maximum {1}")]
    ArrayTooLong(usize, usize),

    /// Trailing data after deserialization.
    #[error("trailing data: {0} bytes remaining")]
    TrailingData(usize),
}

impl serde::ser::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl serde::de::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}
