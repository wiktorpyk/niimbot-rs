//! Thin USB serial transport for talking to a NIIMBOT printer: opening the
//! serial port and packet-level send/receive built on top of
//! [`tokio_serial`].

use std::time::Duration;

use log::{debug, warn};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_serial::{SerialPortBuilderExt, SerialStream};

use crate::error::{AppError, Result};
use crate::protocol::{NiimbotPacket, PacketDecoder, PrinterInfoType, RequestCommandId};

/// Baud rate used for the USB serial link. NIIMBOT printers don't strictly
/// enforce a specific rate over USB (it's a virtual serial port over a CDC
/// interface), but a value must still be supplied to open the port.
const BAUD_RATE: u32 = 115200;

/// Open the USB serial port for a connected printer.
///
/// `port_path` is the OS-specific serial device for the printer's USB
/// connection, e.g. `/dev/ttyUSB0`/`/dev/ttyACM0` on Linux, `/dev/tty.usb*`
/// on macOS, or `COM5` on Windows. The printer must be plugged in via USB
/// and turned off (charge mode) for this port to appear/respond.
pub fn open_port(port_path: &str) -> Result<SerialStream> {
    tokio_serial::new(port_path, BAUD_RATE)
        .timeout(Duration::from_secs(1))
        .open_native_async()
        .map_err(AppError::from)
}

/// A connected NIIMBOT printer: an open serial port, ready to send and
/// receive [`NiimbotPacket`]s.
pub struct PrinterConnection {
    port: SerialStream,
    decoder: PacketDecoder,
    protocol_version: u8,
}

impl PrinterConnection {
    /// Wrap an already-open serial port. The protocol version defaults to
    /// `1` until [`Self::set_protocol_version`] is called, e.g. after
    /// negotiating it via a `PrinterInfo` request.
    pub fn connect(port: SerialStream) -> Self {
        Self {
            port,
            decoder: PacketDecoder::new(),
            protocol_version: 1, // Safe fallback.
        }
    }

    /// The currently negotiated protocol version.
    pub fn protocol_version(&self) -> u8 {
        self.protocol_version
    }

    /// Record the negotiated protocol version, e.g. after querying it from
    /// the printer.
    pub fn set_protocol_version(&mut self, version: u8) {
        self.protocol_version = version;
    }

    /// Serialize and send a packet.
    pub async fn send(&mut self, packet: &NiimbotPacket) -> Result<()> {
        debug!("-> {packet}");
        self.port.write_all(&packet.to_bytes()).await?;
        Ok(())
    }

    /// Wait up to `timeout` for the next packet matching `predicate`,
    /// discarding any non-matching packets received along the way.
    ///
    /// Framing errors in the incoming stream are logged and skipped (the
    /// decoder resynchronizes automatically) rather than aborting the wait.
    pub async fn wait_for(
        &mut self,
        timeout: Duration,
        predicate: impl Fn(&NiimbotPacket) -> bool,
    ) -> Result<NiimbotPacket> {
        tokio::time::timeout(timeout, async {
            let mut buf = [0u8; 512];
            loop {
                while let Some(packet) = self.decoder.try_next().transpose() {
                    match packet {
                        Ok(packet) => {
                            debug!("<- {packet}");
                            if predicate(&packet) {
                                return Ok(packet);
                            }
                        }
                        Err(e) => warn!("dropping malformed packet data: {e}"),
                    }
                }

                debug!("waiting for data from printer...");
                let n = self.port.read(&mut buf).await.map_err(AppError::from)?;
                if n == 0 {
                    return Err(AppError::StreamEnded);
                }
                debug!("received {n} bytes");
                self.decoder.feed(&buf[..n]);
            }
        })
        .await?
    }

    /// Send `packet` and wait up to `timeout` for a reply matching
    /// `predicate`.
    pub async fn send_and_wait(
        &mut self,
        packet: &NiimbotPacket,
        timeout: Duration,
        predicate: impl Fn(&NiimbotPacket) -> bool,
    ) -> Result<NiimbotPacket> {
        self.send(packet).await?;
        self.wait_for(timeout, predicate).await
    }

    /// Query a single [`PrinterInfoType`] parameter and return the reply's
    /// raw data payload.
    ///
    /// The printer doesn't reply on a single fixed command byte: instead it
    /// echoes back `0x40 + type` (e.g. a request for type `0x08` gets a
    /// reply on command `0x48`), so the expected reply command is computed
    /// per-request rather than matched against a single
    /// [`ResponseCommandId`] variant.
    ///
    /// Interpreting the returned bytes (e.g. as a string, a little-endian
    /// integer, or a hex-encoded identifier) depends on which
    /// [`PrinterInfoType`] was requested; this method only handles the
    /// request/reply exchange.
    pub async fn get_printer_info(&mut self, info_type: PrinterInfoType, timeout: Duration) -> Result<Vec<u8>> {
        let request = NiimbotPacket::printer_info_typed(info_type);
        let expected_command = u8::from(RequestCommandId::PrinterInfo).wrapping_add(info_type.into());
        let reply = self
            .send_and_wait(&request, timeout, |p| p.command() == expected_command)
            .await?;
        Ok(reply.data().to_vec())
    }
}