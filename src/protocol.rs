//! NIIMBOT printer packet protocol.
//!
//! Wire format:
//!
//! ```text
//! [0x55, 0x55, CMD, LEN, ...DATA, CHECKSUM, 0xAA, 0xAA]
//! ```
//!
//! `CHECKSUM` is `CMD ^ LEN ^ (xor of all DATA bytes)`.

use std::fmt;

/// Packet head marker.
pub const HEAD: [u8; 2] = [0x55, 0x55];
/// Packet tail marker.
pub const TAIL: [u8; 2] = [0xAA, 0xAA];

/// Number of non-data bytes in a packet: head(2) + cmd(1) + len(1) +
/// checksum(1) + tail(2).
const FRAME_OVERHEAD: usize = HEAD.len() + 1 + 1 + 1 + TAIL.len();

/// Errors that can occur while decoding a byte stream into packets.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DecodeError {
    #[error("invalid packet head")]
    InvalidHead,
    #[error("invalid packet tail")]
    InvalidTail,
    #[error("checksum mismatch: expected {expected:#04x}, got {actual:#04x}")]
    ChecksumMismatch { expected: u8, actual: u8 },
}

/// Request command IDs (client -> printer).
///
/// Only the ones needed for the connect + heartbeat handshake are modeled
/// here; the full NIIMBOT protocol has many more (label rendering, print
/// control, etc.) that this crate doesn't implement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RequestCommandId {
    /// Request general printer parameters (see [`NiimbotPacket::printer_info`]).
    PrinterInfo = 0x40,
    /// Initiate the connect handshake.
    Connect = 0xC1,
    /// Poll printer status.
    Heartbeat = 0xDC,
}

impl From<RequestCommandId> for u8 {
    fn from(id: RequestCommandId) -> Self {
        id as u8
    }
}

/// Response command IDs (printer -> client) relevant to connect + heartbeat.
///
/// Unrecognized command bytes are preserved via [`ResponseCommandId::Other`]
/// rather than being treated as an error, since the printer may reply with
/// commands this client doesn't otherwise need to understand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseCommandId {
    /// `0xC2` - reply to [`RequestCommandId::Connect`].
    Connect,
    /// `0xDD` - "advanced 1" heartbeat reply.
    HeartbeatAdvanced1,
    /// `0xDE` - "basic" heartbeat reply.
    HeartbeatBasic,
    /// `0xDF` - heartbeat reply of unknown/unspecified format.
    HeartbeatUnknown,
    /// `0xD9` - "advanced 2" heartbeat reply.
    HeartbeatAdvanced2,
    /// `0x40` - printer info response (general parameters).
    PrinterInfo,
    /// `0x4A` - printer protocol version response.
    ProtocolVersion,
    /// Any other command byte, preserved verbatim.
    Other(u8),
}

impl From<u8> for ResponseCommandId {
    fn from(byte: u8) -> Self {
        match byte {
            0xC2 => Self::Connect,
            0xDD => Self::HeartbeatAdvanced1,
            0xDE => Self::HeartbeatBasic,
            0xDF => Self::HeartbeatUnknown,
            0xD9 => Self::HeartbeatAdvanced2,
            0x40 => Self::PrinterInfo,
            0x4A => Self::ProtocolVersion,
            other => Self::Other(other),
        }
    }
}

impl ResponseCommandId {
    /// True for any of the heartbeat reply variants, regardless of format.
    pub fn is_heartbeat_reply(self) -> bool {
        matches!(
            self,
            Self::HeartbeatAdvanced1 | Self::HeartbeatBasic | Self::HeartbeatUnknown | Self::HeartbeatAdvanced2
        )
    }
}

/// Result byte carried in a [`ResponseCommandId::Connect`] reply's data
/// payload, indicating the connection state the printer established (or
/// didn't) in response to a [`NiimbotPacket::connect`] request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ConnectResult {
    Disconnect = 0,
    Connected = 1,
    ConnectedNew = 2,
    ConnectedV3 = 3,
}

impl ConnectResult {
    /// Decode a raw result byte, returning `None` for unrecognized values.
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0 => Some(Self::Disconnect),
            1 => Some(Self::Connected),
            2 => Some(Self::ConnectedNew),
            3 => Some(Self::ConnectedV3),
            _ => None,
        }
    }

    /// True for any variant indicating an established connection (i.e.
    /// anything other than [`Self::Disconnect`]).
    pub fn is_connected(self) -> bool {
        !matches!(self, Self::Disconnect)
    }
}

impl From<ConnectResult> for u8 {
    fn from(result: ConnectResult) -> Self {
        result as u8
    }
}

impl fmt::Display for ConnectResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Disconnect => "Disconnect",
            Self::Connected => "Connected",
            Self::ConnectedNew => "ConnectedNew",
            Self::ConnectedV3 => "ConnectedV3",
        };
        write!(f, "{} - {label}", *self as u8)
    }
}

/// Decode a [`ResponseCommandId::Connect`] reply's payload into a
/// [`ConnectResult`].
///
/// Returns `None` if `packet` isn't a `Connect` reply, has an empty
/// payload, or its result byte isn't a recognized [`ConnectResult`] value.
pub fn decode_connect(packet: &NiimbotPacket) -> Option<ConnectResult> {
    if !matches!(packet.response_id(), ResponseCommandId::Connect) {
        return None;
    }
    ConnectResult::from_byte(*packet.data().first()?)
}

/// Sent with [`RequestCommandId::PrinterInfo`] as the single data byte to
/// select which printer parameter is being queried.
///
/// See [`NiimbotPacket::printer_info`] and
/// [`PrinterConnection::get_printer_info`](crate::serial_client::PrinterConnection::get_printer_info).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PrinterInfoType {
    Density = 1,
    Speed = 2,
    LabelType = 3,
    Language = 6,
    AutoShutdownTime = 7,
    /// See the printer model table in the reference implementation.
    PrinterModelId = 8,
    SoftWareVersion = 9,
    BatteryChargeLevel = 10,
    SerialNumber = 11,
    HardWareVersion = 12,
    BluetoothAddress = 13,
    PrintMode = 14,
    Area = 15,
}

impl From<PrinterInfoType> for u8 {
    fn from(t: PrinterInfoType) -> Self {
        t as u8
    }
}

/// Sub-type sent as the single data byte of a [`RequestCommandId::Heartbeat`]
/// request; determines which reply format the printer uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum HeartbeatType {
    Advanced1 = 1,
    Basic = 2,
    Unknown = 3,
    Advanced2 = 4,
}

impl From<HeartbeatType> for u8 {
    fn from(t: HeartbeatType) -> Self {
        t as u8
    }
}

/// A NIIMBOT protocol packet.
///
/// Construct one either via [`NiimbotPacket::new`] for generic use, or one
/// of the request-specific constructors ([`NiimbotPacket::connect`],
/// [`NiimbotPacket::printer_info`], [`NiimbotPacket::heartbeat`]), which set
/// up the correct data payload (and, for `Connect`, the wire prefix quirk).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NiimbotPacket {
    command: u8,
    data: Vec<u8>,
    /// Extra byte prepended before the packet head when serialized. Only
    /// `Connect` requests use this, so it defaults to `None`.
    wire_prefix: Option<u8>,
}

impl NiimbotPacket {
    /// Build a packet from a raw command byte and payload.
    pub fn new(command: impl Into<u8>, data: impl Into<Vec<u8>>) -> Self {
        Self::try_new(command, data).unwrap_or_else(|len| {
            panic!(
                "NiimbotPacket data payload too large: {len} bytes"
            )
        })
    }

    /// Fallible version of [`Self::new`].
    ///
    /// Returns `Err(len)` with the offending payload length if `data` is
    /// longer than 255 bytes, instead of panicking.
    pub fn try_new(command: impl Into<u8>, data: impl Into<Vec<u8>>) -> Result<Self, usize> {
        let data = data.into();
        if data.len() > u8::MAX as usize {
            return Err(data.len());
        }
        Ok(Self {
            command: command.into(),
            data,
            wire_prefix: None,
        })
    }

    /// Build the `Connect` handshake request.
    pub fn connect() -> Self {
        Self {
            command: RequestCommandId::Connect.into(),
            data: vec![1],
            wire_prefix: Some(0x03),
        }
    }

    /// Build a `PrinterInfo` request for the given raw parameter keys.
    ///
    /// Most callers should prefer [`Self::printer_info_typed`], which takes
    /// a [`PrinterInfoType`] and matches the wire format (a single type
    /// byte) used by real printers. This raw form remains available for
    /// querying keys not covered by [`PrinterInfoType`].
    pub fn printer_info(keys: &[u8]) -> Self {
        Self::new(RequestCommandId::PrinterInfo, keys.to_vec())
    }

    /// Build a `PrinterInfo` request for a specific [`PrinterInfoType`].
    pub fn printer_info_typed(info_type: PrinterInfoType) -> Self {
        Self::printer_info(&[info_type.into()])
    }

    /// Build a `Heartbeat` request of the given sub-type.
    pub fn heartbeat(kind: HeartbeatType) -> Self {
        Self::new(RequestCommandId::Heartbeat, vec![kind.into()])
    }

    /// The raw command byte. Mainly useful for tests and callers that want
    /// to inspect packets directly; ordinary reply handling should prefer
    /// [`Self::response_id`].
    pub fn command(&self) -> u8 {
        self.command
    }

    /// The packet's data payload.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Interpret [`Self::command`] as a [`ResponseCommandId`].
    pub fn response_id(&self) -> ResponseCommandId {
        ResponseCommandId::from(self.command)
    }

    /// Compute the packet's checksum: `command ^ len ^ (xor of all data bytes)`.
    fn checksum(&self) -> u8 {
        let len = self.data.len() as u8;
        self.data
            .iter()
            .fold(self.command ^ len, |acc, byte| acc ^ byte)
    }

    /// Serialize this packet to its wire representation.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(FRAME_OVERHEAD + self.data.len() + 1);
        out.extend(self.wire_prefix);
        out.extend_from_slice(&HEAD);
        out.push(self.command);
        out.push(self.data.len() as u8);
        out.extend_from_slice(&self.data);
        out.push(self.checksum());
        out.extend_from_slice(&TAIL);
        out
    }

    /// Try to parse a single packet from the front of `buf`.
    ///
    /// Returns:
    /// - `Ok(Some((packet, consumed)))` on success, where `consumed` is the
    ///   number of bytes of `buf` the packet occupied.
    /// - `Ok(None)` if `buf` starts with a valid head but doesn't yet
    ///   contain a complete packet (i.e. more bytes are needed).
    /// - `Err(_)` if `buf` cannot possibly start a valid packet (bad
    ///   head/tail/checksum).
    pub fn parse(buf: &[u8]) -> Result<Option<(Self, usize)>, DecodeError> {
        if buf.len() < HEAD.len() {
            return Ok(None);
        }
        if buf[..HEAD.len()] != HEAD {
            return Err(DecodeError::InvalidHead);
        }
        if buf.len() < FRAME_OVERHEAD {
            return Ok(None);
        }

        let command = buf[2];
        let data_len = buf[3] as usize;
        let total_len = FRAME_OVERHEAD + data_len;

        if buf.len() < total_len {
            return Ok(None);
        }

        let data = &buf[4..4 + data_len];
        let checksum = buf[4 + data_len];
        let tail = &buf[4 + data_len + 1..total_len];

        if tail != TAIL {
            return Err(DecodeError::InvalidTail);
        }

        let packet = Self::new(command, data.to_vec());
        let expected = packet.checksum();
        if expected != checksum {
            return Err(DecodeError::ChecksumMismatch {
                expected,
                actual: checksum,
            });
        }

        Ok(Some((packet, total_len)))
    }
}

impl fmt::Display for NiimbotPacket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NiimbotPacket {{ command: {:#04x}, data: [", self.command)?;
        for (i, b) in self.data.iter().enumerate() {
            if i > 0 {
                write!(f, " ")?;
            }
            write!(f, "{b:02x}")?;
        }
        write!(f, "] }}")
    }
}

/// Incrementally decodes a byte stream (e.g. successive serial-port reads)
/// into [`NiimbotPacket`]s, buffering partial data between calls.
#[derive(Debug, Default)]
pub struct PacketDecoder {
    buf: Vec<u8>,
}

impl PacketDecoder {
    /// Create an empty decoder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append newly-received bytes to the internal buffer.
    pub fn feed(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    /// Try to decode and remove the next complete packet from the buffer.
    ///
    /// Returns `Ok(None)` if the buffer doesn't yet hold a full packet. On a
    /// framing error the malformed prefix is discarded so the decoder can
    /// resynchronize on subsequent data.
    pub fn try_next(&mut self) -> Result<Option<NiimbotPacket>, DecodeError> {
        match NiimbotPacket::parse(&self.buf) {
            Ok(Some((packet, consumed))) => {
                self.buf.drain(..consumed);
                Ok(Some(packet))
            }
            Ok(None) => Ok(None),
            Err(e) => {
                self.buf.clear();
                Err(e)
            }
        }
    }
}

/// Decoded heartbeat payload. Field availability depends on the reply
/// format the printer used (see [`decode_heartbeat`]).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct HeartbeatData {
    pub lid_closed: Option<bool>,
    pub charge_level: Option<u8>,
    pub paper_inserted: Option<bool>,
    pub paper_rfid_success: Option<bool>,
    pub ribbon_rfid_success: Option<bool>,
    pub ribbon_inserted: Option<bool>,
    pub temp: Option<u8>,
    pub wifi_rssi: Option<u8>,
}

/// Decode a heartbeat reply packet's payload.
///
/// Mirrors `Abstraction.processHeartbeatAdvanced1` /
/// `processHeartbeatAdvanced2` from the reference implementation: the
/// `Advanced1` layout is disambiguated purely by payload length, while
/// `Advanced2` has a fixed layout with optional trailing fields (unused
/// here since our reply doesn't include them).
///
/// Returns `None` if `packet` isn't a recognized heartbeat reply, or its
/// payload length doesn't match any known layout.
pub fn decode_heartbeat(packet: &NiimbotPacket) -> Option<HeartbeatData> {
    let d = packet.data();

    match packet.response_id() {
        ResponseCommandId::HeartbeatAdvanced1 => {
            let mut info = HeartbeatData::default();
            match d.len() {
                10 => {
                    info.lid_closed = Some(d[8] == 0);
                    info.charge_level = Some(d[9]);
                }
                13 => {
                    info.lid_closed = Some(d[9] == 0);
                    info.charge_level = Some(d[10]);
                    info.paper_inserted = Some(d[11] == 0);
                    info.paper_rfid_success = Some(d[12] != 0);
                }
                19 => {
                    info.lid_closed = Some(d[15] == 0);
                    info.charge_level = Some(d[16]);
                    info.paper_inserted = Some(d[17] == 0);
                    info.paper_rfid_success = Some(d[18] != 0);
                }
                20 => {
                    info.paper_inserted = Some(d[18] == 0);
                    info.paper_rfid_success = Some(d[19] != 0);
                }
                _ => return None,
            }
            Some(info)
        }
        ResponseCommandId::HeartbeatAdvanced2 if d.len() >= 9 => Some(HeartbeatData {
            charge_level: Some(d[2]),
            temp: Some(d[3]),
            lid_closed: Some(d[4] == 0),
            paper_inserted: Some(d[5] == 0),
            paper_rfid_success: Some(d[6] != 0),
            ribbon_rfid_success: Some(d[7] != 0),
            ribbon_inserted: Some(d[8] == 0),
            wifi_rssi: if d.len() >= 10 { Some(d[9]) } else { None },
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two back-to-back packets taken verbatim from `packet_parser.ts`'s own
    /// doc comment, used upstream as the canonical bundling example:
    /// `55554a01044faaaa5555f60101f6aaaa`.
    const BUNDLE: [u8; 16] = [
        0x55, 0x55, 0x4a, 0x01, 0x04, 0x4f, 0xaa, 0xaa, //
        0x55, 0x55, 0xf6, 0x01, 0x01, 0xf6, 0xaa, 0xaa,
    ];

    #[test]
    fn parses_reference_bundle() {
        let (p1, consumed1) = NiimbotPacket::parse(&BUNDLE).unwrap().unwrap();
        assert_eq!(consumed1, 8);
        assert_eq!(p1.command(), 0x4a);
        assert_eq!(p1.data(), &[0x04]);

        let (p2, consumed2) = NiimbotPacket::parse(&BUNDLE[consumed1..]).unwrap().unwrap();
        assert_eq!(consumed2, 8);
        assert_eq!(p2.command(), 0xf6);
        assert_eq!(p2.data(), &[0x01]);
    }

    #[test]
    fn decoder_handles_fragmented_feed() {
        let mut decoder = PacketDecoder::new();

        // Feed one byte at a time to exercise partial-buffer handling.
        for byte in &BUNDLE[..10] {
            decoder.feed(&[*byte]);
        }
        let first = decoder.try_next().unwrap().unwrap();
        assert_eq!(first.command(), 0x4a);

        // Second packet is still incomplete.
        assert!(decoder.try_next().unwrap().is_none());

        decoder.feed(&BUNDLE[10..]);
        let second = decoder.try_next().unwrap().unwrap();
        assert_eq!(second.command(), 0xf6);
    }

    #[test]
    fn round_trips_connect_and_heartbeat() {
        let connect = NiimbotPacket::connect();
        let bytes = connect.to_bytes();
        // 0x03 wire-prefix quirk, then a normal packet.
        assert_eq!(bytes[0], 0x03);
        let (parsed, consumed) = NiimbotPacket::parse(&bytes[1..]).unwrap().unwrap();
        assert_eq!(consumed, bytes.len() - 1);
        assert_eq!(parsed.command(), RequestCommandId::Connect.into());
        assert_eq!(parsed.data(), &[1]);

        let heartbeat = NiimbotPacket::heartbeat(HeartbeatType::Advanced1);
        let bytes = heartbeat.to_bytes();
        assert_eq!(bytes[0], HEAD[0]); // no prefix quirk for heartbeat
        let (parsed, _) = NiimbotPacket::parse(&bytes).unwrap().unwrap();
        assert_eq!(parsed.data(), &[HeartbeatType::Advanced1 as u8]);
    }

    #[test]
    fn rejects_bad_checksum() {
        let mut bytes = NiimbotPacket::heartbeat(HeartbeatType::Advanced1).to_bytes();
        let checksum_pos = bytes.len() - 3; // just before TAIL
        bytes[checksum_pos] ^= 0xff;
        assert_eq!(
            NiimbotPacket::parse(&bytes),
            Err(DecodeError::ChecksumMismatch {
                expected: bytes[checksum_pos] ^ 0xff,
                actual: bytes[checksum_pos],
            })
        );
    }

    #[test]
    fn incomplete_packet_returns_none_not_error() {
        let bytes = NiimbotPacket::heartbeat(HeartbeatType::Advanced1).to_bytes();
        for cut in 0..bytes.len() {
            assert_eq!(NiimbotPacket::parse(&bytes[..cut]), Ok(None));
        }
    }

    #[test]
    fn decode_heartbeat_advanced1_len_10() {
        let mut data = vec![0u8; 10];
        data[8] = 0; // lid closed
        data[9] = 77; // charge level
        let packet = NiimbotPacket::new(0xDDu8, data);
        let decoded = decode_heartbeat(&packet).unwrap();
        assert_eq!(decoded.lid_closed, Some(true));
        assert_eq!(decoded.charge_level, Some(77));
    }

    #[test]
    fn decode_heartbeat_advanced2() {
        let data = vec![0, 0, 88, 25, 0, 1, 1, 1, 1];
        let packet = NiimbotPacket::new(0xD9u8, data);
        let decoded = decode_heartbeat(&packet).unwrap();
        assert_eq!(decoded.charge_level, Some(88));
        assert_eq!(decoded.temp, Some(25));
        assert_eq!(decoded.lid_closed, Some(true));
        assert_eq!(decoded.paper_inserted, Some(false));
        assert_eq!(decoded.paper_rfid_success, Some(true));
        assert_eq!(decoded.ribbon_rfid_success, Some(true));
        assert_eq!(decoded.ribbon_inserted, Some(false));
        assert_eq!(decoded.wifi_rssi, None);
    }

    #[test]
    fn try_new_rejects_oversized_payload() {
        let data = vec![0u8; 256];
        assert_eq!(NiimbotPacket::try_new(0xD9u8, data), Err(256));
    }

    #[test]
    fn try_new_accepts_max_len_payload() {
        let data = vec![0u8; 255];
        assert!(NiimbotPacket::try_new(0xD9u8, data).is_ok());
    }

    #[test]
    #[should_panic(expected = "too large")]
    fn new_panics_on_oversized_payload() {
        let data = vec![0u8; 300];
        let _ = NiimbotPacket::new(0xD9u8, data);
    }

    #[test]
    fn printer_info_typed_matches_raw_single_key() {
        let typed = NiimbotPacket::printer_info_typed(PrinterInfoType::SerialNumber);
        let raw = NiimbotPacket::printer_info(&[PrinterInfoType::SerialNumber.into()]);
        assert_eq!(typed, raw);
        assert_eq!(typed.data(), &[11]);
    }

    #[test]
    fn decode_connect_rejects_unknown_byte_and_wrong_command() {
        let unknown = NiimbotPacket::new(0xC2u8, vec![99]);
        assert_eq!(decode_connect(&unknown), None);

        let wrong_command = NiimbotPacket::new(0xDDu8, vec![1]);
        assert_eq!(decode_connect(&wrong_command), None);

        let empty = NiimbotPacket::new(0xC2u8, vec![]);
        assert_eq!(decode_connect(&empty), None);
    }

    #[test]
    fn decode_heartbeat_advanced2_with_wifi() {
        let data = vec![0, 0, 88, 25, 0, 1, 1, 1, 1, 42];
        let packet = NiimbotPacket::new(0xD9u8, data);
        let decoded = decode_heartbeat(&packet).unwrap();
        assert_eq!(decoded.charge_level, Some(88));
        assert_eq!(decoded.temp, Some(25));
        assert_eq!(decoded.lid_closed, Some(true));
        assert_eq!(decoded.paper_inserted, Some(false));
        assert_eq!(decoded.paper_rfid_success, Some(true));
        assert_eq!(decoded.ribbon_rfid_success, Some(true));
        assert_eq!(decoded.ribbon_inserted, Some(false));
        assert_eq!(decoded.wifi_rssi, Some(42));
    }
}