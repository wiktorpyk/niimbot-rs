//! A small library for talking to NIIMBOT label printers over USB serial.
//!
//! This crate provides:
//!
//! - [`protocol`]: the wire packet format, request/response command IDs, and
//!   a stream decoder for reassembling packets from raw bytes.
//! - [`serial_client`]: a thin transport layer on top of [`tokio_serial`]
//!   that opens the printer's serial port and sends/receives
//!   [`protocol::NiimbotPacket`]s.
//! - [`error`]: the crate's unified error type.
//!
//! This crate only models the small slice of the NIIMBOT protocol needed
//! for a connect handshake and periodic heartbeats; it does not implement
//! label rendering or printing. See `examples/heartbeat_monitor.rs` for a
//! complete example that connects to a printer and polls its status.

pub mod error;
pub mod protocol;
pub mod serial_client;

pub use error::{AppError, Result};
pub use serial_client::{detect_printer_port, open_port, PrinterConnection};
