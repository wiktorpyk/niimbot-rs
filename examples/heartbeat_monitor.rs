//! Example: connect to a NIIMBOT printer over USB serial, perform the
//! connect handshake, negotiate the protocol version, and then poll the
//! printer's status with periodic heartbeats, logging each reply.
//!
//! Run with:
//!
//! ```text
//! cargo run --example heartbeat_monitor
//! ```

use std::time::Duration;

use log::info;

use niimbot_rs::error::Result;
use niimbot_rs::protocol::{decode_heartbeat, HeartbeatType, NiimbotPacket, ResponseCommandId};
use niimbot_rs::serial_client::{detect_printer_port, open_port, PrinterConnection};

/// How long to wait for a reply to any single request.
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(5);
/// Timeout for the initial connect handshake.
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);
/// Interval between heartbeat requests once connected.
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(2);

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

    let connect_reply = printer
        .send_and_wait(&NiimbotPacket::connect(), HANDSHAKE_TIMEOUT, |p| {
            matches!(p.response_id(), ResponseCommandId::Connect)
        })
        .await?;
    info!("handshake complete, result byte = {:?}", connect_reply.data().first());

    negotiate_protocol_version(&mut printer).await;

    run_heartbeat_loop(&mut printer).await
}

/// Query the printer for its protocol version (`PrinterInfo` key 10 /
/// `0x0a`) and record it on `printer`. If the query fails, or the printer's
/// reply doesn't include the version key, the connection's default version
/// (`1`) is left in place.
async fn negotiate_protocol_version(printer: &mut PrinterConnection) {
    const PROTOCOL_VERSION_KEY: u8 = 10;

    let info_req = NiimbotPacket::printer_info(&[PROTOCOL_VERSION_KEY]);
    let reply = printer
        .send_and_wait(&info_req, RESPONSE_TIMEOUT, |p| {
            matches!(
                p.response_id(),
                ResponseCommandId::PrinterInfo | ResponseCommandId::ProtocolVersion
            )
        })
        .await;

    let reply = match reply {
        Ok(reply) => reply,
        Err(e) => {
            info!("failed to query printer info, defaulting to protocol version 1: {e}");
            return;
        }
    };

    let version = match reply.response_id() {
        ResponseCommandId::ProtocolVersion => reply.data().first().copied(),
        ResponseCommandId::PrinterInfo => find_tlv_value(reply.data(), PROTOCOL_VERSION_KEY),
        _ => None,
    };

    match version {
        Some(v) => {
            info!("negotiated protocol version: {v}");
            printer.set_protocol_version(v);
        }
        None => info!("printer replied but protocol version (key 10) was not found, defaulting to 1"),
    }
}

/// Scan a `PrinterInfo` reply's key/length/value-encoded payload for `key`
/// and return its first byte, if present.
fn find_tlv_value(data: &[u8], key: u8) -> Option<u8> {
    let mut i = 0;
    while i + 1 < data.len() {
        let entry_key = data[i];
        let len = data[i + 1] as usize;
        let value_start = i + 2;
        if value_start + len > data.len() {
            break;
        }
        let value = &data[value_start..value_start + len];
        if entry_key == key {
            return value.first().copied();
        }
        i = value_start + len;
    }
    None
}

/// Send a heartbeat every [`HEARTBEAT_INTERVAL`] and print the decoded reply directly to console.
/// Individual heartbeat failures (timeout, transport error) are printed to stderr and
/// do not stop the loop.
async fn run_heartbeat_loop(printer: &mut PrinterConnection) -> Result<()> {
    let mut interval = tokio::time::interval(HEARTBEAT_INTERVAL);

    loop {
        interval.tick().await;

        let heartbeat_type = if printer.protocol_version() >= 3 {
            HeartbeatType::Advanced2
        } else {
            HeartbeatType::Advanced1
        };

        let request = NiimbotPacket::heartbeat(heartbeat_type);
        let reply = printer
            .send_and_wait(&request, RESPONSE_TIMEOUT, |p| p.response_id().is_heartbeat_reply())
            .await;

        // Using direct print macros instead of log macros so these
        // messages bypass the RUST_LOG filters and always show up.
        match reply {
            Ok(packet) => match decode_heartbeat(&packet) {
                Some(data) => println!("heartbeat ok: {data:?}"),
                None => println!("heartbeat ok (payload not decodable, {} bytes)", packet.data().len()),
            },
            Err(e) => eprintln!("heartbeat failed: {e}"),
        }
    }
}