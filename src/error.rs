//! Unified error type for this crate.

use crate::protocol::DecodeError;

/// Errors that can occur while opening a printer connection or exchanging
/// packets with it.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// Failed to open or configure the underlying serial port.
    #[error("serial port error: {0}")]
    Serial(#[from] tokio_serial::Error),

    /// Failed to enumerate serial ports during auto-detection.
    #[error("serial port enumeration error: {0}")]
    SerialPortEnum(serialport::Error),

    /// A read or write on the open serial port failed.
    #[error("i/o error: {0}")]
    Io(std::io::Error),

    /// The printer's USB connection was physically lost — the device was
    /// unplugged, lost power, or otherwise disappeared from the bus. This is
    /// detected from `EIO`/`ENXIO` on a serial read/write, which is what the
    /// underlying `ttyACM`/`ttyUSB` device reliably produces once the kernel
    /// notices it's gone.
    ///
    /// Unlike other I/O errors, this is not worth retrying against the same
    /// [`crate::serial_client::PrinterConnection`]: the port itself is dead,
    /// so callers should stop using this connection rather than resend
    /// anything on it. Re-establishing a connection requires detecting the
    /// port again (e.g. via [`crate::serial_client::detect_printer_port`])
    /// and opening it from scratch.
    #[error("printer unplugged: {0}")]
    PrinterUnplugged(std::io::Error),

    /// A request/reply exchange failed and, after waiting and resending the
    /// `Connect` handshake, the printer reported a disconnected state (or
    /// sent no usable reply at all). The serial port itself is still
    /// present, but the printer session could not be recovered.
    #[error("printer connection could not be recovered")]
    ConnectionLost,

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

    /// No NIIMBOT printer was found during auto-detection.
    #[error("no NIIMBOT printer found")]
    NoPrinterFound,
}

/// OS error code for `EIO`, returned by the kernel for reads/writes on a
/// USB serial device that has disappeared (unplugged, lost power, etc.).
const EIO: i32 = 5;
/// OS error code for `ENXIO`, occasionally returned instead of `EIO` for the
/// same "device is gone" condition, depending on driver/platform.
const ENXIO: i32 = 6;

impl From<std::io::Error> for AppError {
    /// Classifies `EIO`/`ENXIO` as [`AppError::PrinterUnplugged`] (the
    /// device is physically gone and this connection cannot be reused) and
    /// everything else as the generic [`AppError::Io`] (potentially
    /// transient and worth retrying).
    fn from(e: std::io::Error) -> Self {
        match e.raw_os_error() {
            Some(EIO) | Some(ENXIO) => AppError::PrinterUnplugged(e),
            _ => AppError::Io(e),
        }
    }
}

/// Convenience alias for `Result<T, AppError>`.
pub type Result<T> = std::result::Result<T, AppError>;