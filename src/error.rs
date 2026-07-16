//! Unified error type for this crate.

use crate::protocol::DecodeError;

/// Errors that can occur while opening a printer connection or exchanging
/// packets with it.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// Failed to open or configure the underlying serial port.
    #[error("serial port error: {0}")]
    Serial(#[from] tokio_serial::Error),

    /// A read or write on the open serial port failed.
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),

    /// Bytes received from the printer didn't form a valid packet.
    #[error("malformed packet: {0}")]
    Decode(#[from] DecodeError),

    /// No matching reply arrived before the deadline passed.
    #[error("timed out waiting for response")]
    Timeout(#[from] tokio::time::error::Elapsed),

    /// The serial connection was closed (zero-byte read) while waiting for
    /// a reply.
    #[error("notification stream ended unexpectedly")]
    StreamEnded,
}

/// Convenience alias for `Result<T, AppError>`.
pub type Result<T> = std::result::Result<T, AppError>;
