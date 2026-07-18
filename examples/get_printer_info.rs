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

use std::time::Duration;

use log::info;

use niimbot_rs::error::Result;
use niimbot_rs::protocol::{decode_printer_info, NiimbotPacket, PrinterInfoType, ResponseCommandId};
use niimbot_rs::serial_client::{detect_printer_port, open_port, PrinterConnection};

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

    let printer_port = std::env::var("PRINTER_PORT")
        .ok()
        .or_else(|| detect_printer_port().ok())
        .expect("Failed to detect printer port. Set PRINTER_PORT environment variable or ensure printer is connected.");
    
    info!("opening printer serial port at {printer_port}...");
    let port = open_port(&printer_port)?;

    let mut printer = PrinterConnection::connect(port);
    info!("connected, sending handshake...");

    let connect_response = printer
        .send_and_wait(&NiimbotPacket::connect(), HANDSHAKE_TIMEOUT, |p| {
            matches!(p.response_id(), ResponseCommandId::Connect)
        })
        .await?;
    printer.note_connect_reply(&connect_response);
    match niimbot_rs::protocol::decode_connect(&connect_response) {
        Some(result) => println!("Connect response code: {result}"),
        None => println!("Connect response code: {:02x?} (unrecognized)", connect_response.data()),
    }
    info!("handshake complete");

    for info_type in INFO_TYPES {
        match printer.get_printer_info(*info_type, RESPONSE_TIMEOUT).await {
            Ok(data) => println!("{info_type:?}: {}", decode_printer_info(*info_type, &data)),
            Err(e) => eprintln!("{info_type:?}: failed ({e})"),
        }
    }

    Ok(())
}
