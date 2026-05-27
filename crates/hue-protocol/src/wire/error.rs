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
