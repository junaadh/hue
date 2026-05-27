use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ProtocolError {
    InvalidMagic = 0x01,
    InvalidVersion = 0x02,
    UnknownMessageType = 0x03,
    InvalidLength = 0x04,
    OversizedPayload = 0x05,
    InvalidUtf8 = 0x06,
    CrcMismatch = 0x07,
    InvalidFieldMask = 0x08,
    ReservedNonZero = 0x09,
    ArtworkIdMismatch = 0x0A,
    ChunkIndexMismatch = 0x0B,
    ChecksumMismatch = 0x0C,
    EpochOverflow = 0x0D,
    InvalidAlignment = 0x0E,
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let msg = match self {
            Self::InvalidMagic => "invalid magic",
            Self::InvalidVersion => "invalid version",
            Self::UnknownMessageType => "unknown message type",
            Self::InvalidLength => "invalid length",
            Self::OversizedPayload => "oversized payload",
            Self::InvalidUtf8 => "invalid utf-8",
            Self::CrcMismatch => "crc mismatch",
            Self::InvalidFieldMask => "invalid field mask",
            Self::ReservedNonZero => "reserved field/value non-zero",
            Self::ArtworkIdMismatch => "artwork id mismatch",
            Self::ChunkIndexMismatch => "chunk index mismatch",
            Self::ChecksumMismatch => "checksum mismatch",
            Self::EpochOverflow => "epoch overflow",
            Self::InvalidAlignment => "invalid alignment",
        };

        f.write_str(msg)
    }
}

impl core::error::Error for ProtocolError {}
