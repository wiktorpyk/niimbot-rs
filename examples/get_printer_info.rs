//! Example: connect to a NIIMBOT printer over USB serial, perform the
//! connect handshake, and then query a handful of `PrinterInfo` parameters
//! (serial number, firmware/hardware version, battery level, ...), printing
//! each raw reply.
//!
//! Run with:
//!
//! ```text
//! cargo run --example get_printer_info
//! ```
//!
//! Adjust [`PRINTER_PORT`] below to match your printer's serial device.

use std::time::Duration;

use log::info;

use niimbot_rs::error::Result;
use niimbot_rs::protocol::{NiimbotPacket, PrinterInfoType, ResponseCommandId};
use niimbot_rs::serial_client::{open_port, PrinterConnection};

/// USB serial device for the printer's connection.
const PRINTER_PORT: &str = "/dev/ttyACM0";
/// How long to wait for a reply to any single request.
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(5);
/// Timeout for the initial connect handshake.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);

/// `PrinterInfo` parameters queried by this example, in the order they're
/// requested.
const INFO_TYPES: &[PrinterInfoType] = &[
    PrinterInfoType::PrinterModelId,
    PrinterInfoType::SerialNumber,
    PrinterInfoType::SoftWareVersion,
    PrinterInfoType::HardWareVersion,
    PrinterInfoType::BluetoothAddress,
    PrinterInfoType::BatteryChargeLevel,
    PrinterInfoType::AutoShutdownTime,
    PrinterInfoType::LabelType,
];

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    info!("opening printer serial port at {PRINTER_PORT}...");
    let port = open_port(PRINTER_PORT)?;

    let mut printer = PrinterConnection::connect(port);
    info!("connected, sending handshake...");

    printer
        .send_and_wait(&NiimbotPacket::connect(), HANDSHAKE_TIMEOUT, |p| {
            matches!(p.response_id(), ResponseCommandId::Connect)
        })
        .await?;
    info!("handshake complete");

    for info_type in INFO_TYPES {
        match printer.get_printer_info(*info_type, RESPONSE_TIMEOUT).await {
            Ok(data) => println!("{info_type:?}: {data:02x?}"),
            Err(e) => eprintln!("{info_type:?}: failed ({e})"),
        }
    }

    Ok(())
}
