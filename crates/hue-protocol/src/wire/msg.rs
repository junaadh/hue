use crate::wire::ProtocolError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MessageType {
    Ping = 0x00,
    Pong = 0x01,
    StateFull = 0x02,
    StateDelta = 0x03,
    ArtworkBegin = 0x04,
    ArtworkChunk = 0x05,
    ArtworkEnd = 0x06,
    ArtworkAbort = 0x07,
    Command = 0x08,
    Ack = 0x09,
    Nack = 0x0A,
    Error = 0x0B,
    Resync = 0x0C,
    Handshake = 0x0D,
}

impl MessageType {
    pub fn from_u8(value: u8) -> Result<Self, ProtocolError> {
        match value {
            0x00 => Ok(Self::Ping),
            0x01 => Ok(Self::Pong),
            0x02 => Ok(Self::StateFull),
            0x03 => Ok(Self::StateDelta),
            0x04 => Ok(Self::ArtworkBegin),
            0x05 => Ok(Self::ArtworkChunk),
            0x06 => Ok(Self::ArtworkEnd),
            0x07 => Ok(Self::ArtworkAbort),
            0x08 => Ok(Self::Command),
            0x09 => Ok(Self::Ack),
            0x0A => Ok(Self::Nack),
            0x0B => Ok(Self::Error),
            0x0C => Ok(Self::Resync),
            0x0D => Ok(Self::Handshake),
            _ => Err(ProtocolError::UnknownMessageType),
        }
    }
}
