//! Request/response command ID enums.
//!
//! Split out from the rest of [`crate::protocol`] because this is the part
//! of the protocol that's just a big, flat ID table (mirroring
//! `commands.ts` in the reference implementation) rather than logic.

/// Request command IDs (client -> printer).
///
/// This mirrors `RequestCommandId` from the reference implementation's
/// `commands.ts` (minus `Invalid`, which isn't representable as a `u8`).
/// Only a handful of these are currently used elsewhere in this crate (see
/// [`crate::protocol::NiimbotPacket`]'s constructors); the rest are
/// provided so callers can build/recognize other request types without
/// having to fall back to raw bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RequestCommandId {
    /// Entire packet should be prefixed with `0x03` on the wire.
    Connect = 0xC1,
    CancelPrint = 0xDA,
    CalibrateHeight = 0x59,
    Heartbeat = 0xDC,
    LabelPositioningCalibration = 0x8E,
    PageEnd = 0xE3,
    PrinterLog = 0x05,
    PageStart = 0x03,
    PrintBitmapRow = 0x85,
    /// Sent instead of [`Self::PrintBitmapRow`] if black pixels < 6.
    PrintBitmapRowIndexed = 0x83,
    PrintClear = 0x20,
    PrintEmptyRow = 0x84,
    PrintEnd = 0xF3,
    /// Request general printer parameters (see
    /// [`crate::protocol::NiimbotPacket::printer_info`]).
    PrinterInfo = 0x40,
    PrinterConfig = 0xAF,
    PrinterStatusData = 0xA5,
    PrinterReset = 0x28,
    PrintQuantity = 0x15,
    PrintStart = 0x01,
    PrintStatus = 0xA3,
    RfidInfo = 0x1A,
    RfidInfo2 = 0x1C,
    RfidSuccessTimes = 0x54,
    SetAutoShutdownTime = 0x27,
    SetDensity = 0x21,
    SetLabelType = 0x23,
    /// 2, 4 or 6 bytes.
    SetPageSize = 0x13,
    SoundSettings = 0x58,
    /// Some info request (niimbot app); `0x01` long, `0x02` short.
    AntiFake = 0x0B,
    WriteRFID = 0x70,
    PrintTestPage = 0x5A,
    StartFirmwareUpgrade = 0xF5,
    FirmwareCrc = 0x91,
    FirmwareCommit = 0x92,
    FirmwareChunk = 0x9B,
    FirmwareNoMoreChunks = 0x9C,
    PrinterCheckLine = 0x86,
    GetCurrentTimeFormat = 0x12,
    PrinterConfig2 = 0x07,
    GetKeyFunction = 0x09,
    GetPrintQuality = 0x0D,
    GetPrinterConfigurationWifi = 0xA2,
}

impl From<RequestCommandId> for u8 {
    fn from(id: RequestCommandId) -> Self {
        id as u8
    }
}

/// Response command IDs (printer -> client) relevant to connect + heartbeat.
///
/// This mirrors `ResponseCommandId` from the reference implementation's
/// `commands.ts` (minus `In_Invalid`). Unrecognized command bytes are
/// preserved via [`ResponseCommandId::Other`] rather than being treated as
/// an error, since the printer may reply with commands this client doesn't
/// otherwise need to understand.
///
/// Note: [`Self::ProtocolVersion`] and [`Self::PrinterInfoChargeLevel`]
/// share the same wire byte (`0x4A` = `0x40` + `PrinterInfoType::BatteryChargeLevel as u8`).
/// The reference implementation only names this byte
/// `In_PrinterInfoChargeLevel`; this crate additionally uses it (queried
/// with `PrinterInfoType`'s key `10`) to read back a protocol version
/// number in the connect handshake, hence the second name. Both names
/// resolve to the same variant/byte - there is no separate charge-level
/// variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResponseCommandId {
    NotSupported,
    /// `0xC2` - reply to [`RequestCommandId::Connect`].
    Connect,
    CalibrateHeight,
    CancelPrint,
    AntiFake,
    /// `0xDD` - "advanced 1" heartbeat reply.
    HeartbeatAdvanced1,
    /// `0xDE` - "basic" heartbeat reply.
    HeartbeatBasic,
    /// `0xDF` - heartbeat reply of unknown/unspecified format.
    HeartbeatUnknown,
    /// `0xD9` - "advanced 2" heartbeat reply.
    HeartbeatAdvanced2,
    LabelPositioningCalibration,
    PageStart,
    PrintClear,
    /// Sent by some printers after [`RequestCommandId::PageEnd`] along with
    /// [`Self::PageEnd`].
    PrinterCheckLine,
    PrintEnd,
    /// `0x40` - printer info response (general parameters).
    PrinterInfo,
    PrinterConfig,
    PrinterLog,
    PrinterInfoAutoShutDownTime,
    PrinterInfoBluetoothAddress,
    PrinterInfoSpeed,
    PrinterInfoDensity,
    PrinterInfoLanguage,
    /// `0x4A` - printer protocol version response. Shares its wire byte
    /// with [`Self::PrinterInfoChargeLevel`]; see the enum-level doc.
    ProtocolVersion,
    /// `0x4A` - same wire byte as [`Self::ProtocolVersion`]; see the
    /// enum-level doc. Included so callers reading raw `PrinterInfoType`
    /// replies can refer to this response by its canonical protocol name.
    PrinterInfoChargeLevel,
    PrinterInfoHardWareVersion,
    PrinterInfoLabelType,
    PrinterInfoPrinterCode,
    PrinterInfoSerialNumber,
    PrinterInfoSoftWareVersion,
    PrinterInfoArea,
    PrinterStatusData,
    PrinterReset,
    PrintStatus,
    /// For example, received after [`RequestCommandId::SetPageSize`] when
    /// page print is not started.
    PrintError,
    PrintQuantity,
    PrintStart,
    RfidInfo,
    RfidInfo2,
    RfidSuccessTimes,
    SetAutoShutdownTime,
    SetDensity,
    SetLabelType,
    SetPageSize,
    SoundSettings,
    PageEnd,
    PrinterPageIndex,
    PrintTestPage,
    WriteRFID,
    StartFirmwareUpgrade,
    RequestFirmwareCrc,
    RequestFirmwareChunk,
    FirmwareCheckResult,
    FirmwareResult,
    /// Sent before [`Self::PrinterCheckLine`].
    ResetTimeout,
    GetCurrentTimeFormat,
    PrinterConfig2,
    GetKeyFunction,
    GetPrintQuality,
    GetPrinterConfigurationWifi,
    /// Any other command byte, preserved verbatim.
    Other(u8),
}

impl From<u8> for ResponseCommandId {
    fn from(byte: u8) -> Self {
        match byte {
            0x00 => Self::NotSupported,
            0xC2 => Self::Connect,
            0x69 => Self::CalibrateHeight,
            0xD0 => Self::CancelPrint,
            0x0C => Self::AntiFake,
            0xDD => Self::HeartbeatAdvanced1,
            0xDE => Self::HeartbeatBasic,
            0xDF => Self::HeartbeatUnknown,
            0xD9 => Self::HeartbeatAdvanced2,
            0x8F => Self::LabelPositioningCalibration,
            0x04 => Self::PageStart,
            0x30 => Self::PrintClear,
            0xD3 => Self::PrinterCheckLine,
            0xF4 => Self::PrintEnd,
            0x40 => Self::PrinterInfo,
            0xBF => Self::PrinterConfig,
            0x06 => Self::PrinterLog,
            0x47 => Self::PrinterInfoAutoShutDownTime,
            0x4D => Self::PrinterInfoBluetoothAddress,
            0x42 => Self::PrinterInfoSpeed,
            0x41 => Self::PrinterInfoDensity,
            0x46 => Self::PrinterInfoLanguage,
            0x4A => Self::ProtocolVersion,
            0x4C => Self::PrinterInfoHardWareVersion,
            0x43 => Self::PrinterInfoLabelType,
            0x48 => Self::PrinterInfoPrinterCode,
            0x4B => Self::PrinterInfoSerialNumber,
            0x49 => Self::PrinterInfoSoftWareVersion,
            0x4F => Self::PrinterInfoArea,
            0xB5 => Self::PrinterStatusData,
            0x38 => Self::PrinterReset,
            0xB3 => Self::PrintStatus,
            0xDB => Self::PrintError,
            0x16 => Self::PrintQuantity,
            0x02 => Self::PrintStart,
            0x1B => Self::RfidInfo,
            0x1D => Self::RfidInfo2,
            0x64 => Self::RfidSuccessTimes,
            0x37 => Self::SetAutoShutdownTime,
            0x31 => Self::SetDensity,
            0x33 => Self::SetLabelType,
            0x14 => Self::SetPageSize,
            0x68 => Self::SoundSettings,
            0xE4 => Self::PageEnd,
            0xE0 => Self::PrinterPageIndex,
            0x6A => Self::PrintTestPage,
            0x71 => Self::WriteRFID,
            0xF6 => Self::StartFirmwareUpgrade,
            0x90 => Self::RequestFirmwareCrc,
            0x9A => Self::RequestFirmwareChunk,
            0x9D => Self::FirmwareCheckResult,
            0x9E => Self::FirmwareResult,
            0xC6 => Self::ResetTimeout,
            0x11 => Self::GetCurrentTimeFormat,
            0x08 => Self::PrinterConfig2,
            0x0A => Self::GetKeyFunction,
            0x0D => Self::GetPrintQuality,
            0xB2 => Self::GetPrinterConfigurationWifi,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_command_id_round_trips_all_variants() {
        // Every request ID should be recoverable via a plain cast, and
        // none should collide with each other.
        let ids = [
            RequestCommandId::Connect,
            RequestCommandId::CancelPrint,
            RequestCommandId::CalibrateHeight,
            RequestCommandId::Heartbeat,
            RequestCommandId::LabelPositioningCalibration,
            RequestCommandId::PageEnd,
            RequestCommandId::PrinterLog,
            RequestCommandId::PageStart,
            RequestCommandId::PrintBitmapRow,
            RequestCommandId::PrintBitmapRowIndexed,
            RequestCommandId::PrintClear,
            RequestCommandId::PrintEmptyRow,
            RequestCommandId::PrintEnd,
            RequestCommandId::PrinterInfo,
            RequestCommandId::PrinterConfig,
            RequestCommandId::PrinterStatusData,
            RequestCommandId::PrinterReset,
            RequestCommandId::PrintQuantity,
            RequestCommandId::PrintStart,
            RequestCommandId::PrintStatus,
            RequestCommandId::RfidInfo,
            RequestCommandId::RfidInfo2,
            RequestCommandId::RfidSuccessTimes,
            RequestCommandId::SetAutoShutdownTime,
            RequestCommandId::SetDensity,
            RequestCommandId::SetLabelType,
            RequestCommandId::SetPageSize,
            RequestCommandId::SoundSettings,
            RequestCommandId::AntiFake,
            RequestCommandId::WriteRFID,
            RequestCommandId::PrintTestPage,
            RequestCommandId::StartFirmwareUpgrade,
            RequestCommandId::FirmwareCrc,
            RequestCommandId::FirmwareCommit,
            RequestCommandId::FirmwareChunk,
            RequestCommandId::FirmwareNoMoreChunks,
            RequestCommandId::PrinterCheckLine,
            RequestCommandId::GetCurrentTimeFormat,
            RequestCommandId::PrinterConfig2,
            RequestCommandId::GetKeyFunction,
            RequestCommandId::GetPrintQuality,
            RequestCommandId::GetPrinterConfigurationWifi,
        ];
        let mut seen = std::collections::HashSet::new();
        for id in ids {
            assert!(seen.insert(u8::from(id)), "duplicate byte for {id:?}");
        }
    }

    #[test]
    fn response_command_id_from_byte_covers_reference_table() {
        let cases: &[(u8, ResponseCommandId)] = &[
            (0x00, ResponseCommandId::NotSupported),
            (0xC2, ResponseCommandId::Connect),
            (0x69, ResponseCommandId::CalibrateHeight),
            (0xD0, ResponseCommandId::CancelPrint),
            (0x0C, ResponseCommandId::AntiFake),
            (0x8F, ResponseCommandId::LabelPositioningCalibration),
            (0x04, ResponseCommandId::PageStart),
            (0x30, ResponseCommandId::PrintClear),
            (0xD3, ResponseCommandId::PrinterCheckLine),
            (0xF4, ResponseCommandId::PrintEnd),
            (0xBF, ResponseCommandId::PrinterConfig),
            (0x06, ResponseCommandId::PrinterLog),
            (0x47, ResponseCommandId::PrinterInfoAutoShutDownTime),
            (0x4D, ResponseCommandId::PrinterInfoBluetoothAddress),
            (0x42, ResponseCommandId::PrinterInfoSpeed),
            (0x41, ResponseCommandId::PrinterInfoDensity),
            (0x46, ResponseCommandId::PrinterInfoLanguage),
            (0x4C, ResponseCommandId::PrinterInfoHardWareVersion),
            (0x43, ResponseCommandId::PrinterInfoLabelType),
            (0x48, ResponseCommandId::PrinterInfoPrinterCode),
            (0x4B, ResponseCommandId::PrinterInfoSerialNumber),
            (0x49, ResponseCommandId::PrinterInfoSoftWareVersion),
            (0x4F, ResponseCommandId::PrinterInfoArea),
            (0xB5, ResponseCommandId::PrinterStatusData),
            (0x38, ResponseCommandId::PrinterReset),
            (0xB3, ResponseCommandId::PrintStatus),
            (0xDB, ResponseCommandId::PrintError),
            (0x16, ResponseCommandId::PrintQuantity),
            (0x02, ResponseCommandId::PrintStart),
            (0x1B, ResponseCommandId::RfidInfo),
            (0x1D, ResponseCommandId::RfidInfo2),
            (0x64, ResponseCommandId::RfidSuccessTimes),
            (0x37, ResponseCommandId::SetAutoShutdownTime),
            (0x31, ResponseCommandId::SetDensity),
            (0x33, ResponseCommandId::SetLabelType),
            (0x14, ResponseCommandId::SetPageSize),
            (0x68, ResponseCommandId::SoundSettings),
            (0xE4, ResponseCommandId::PageEnd),
            (0xE0, ResponseCommandId::PrinterPageIndex),
            (0x6A, ResponseCommandId::PrintTestPage),
            (0x71, ResponseCommandId::WriteRFID),
            (0xF6, ResponseCommandId::StartFirmwareUpgrade),
            (0x90, ResponseCommandId::RequestFirmwareCrc),
            (0x9A, ResponseCommandId::RequestFirmwareChunk),
            (0x9D, ResponseCommandId::FirmwareCheckResult),
            (0x9E, ResponseCommandId::FirmwareResult),
            (0xC6, ResponseCommandId::ResetTimeout),
            (0x11, ResponseCommandId::GetCurrentTimeFormat),
            (0x08, ResponseCommandId::PrinterConfig2),
            (0x0A, ResponseCommandId::GetKeyFunction),
            (0x0D, ResponseCommandId::GetPrintQuality),
            (0xB2, ResponseCommandId::GetPrinterConfigurationWifi),
        ];
        for (byte, expected) in cases {
            assert_eq!(ResponseCommandId::from(*byte), *expected, "byte {byte:#04x}");
        }
    }

    #[test]
    fn unrecognized_byte_falls_back_to_other() {
        assert_eq!(ResponseCommandId::from(0xEE), ResponseCommandId::Other(0xEE));
    }
}
