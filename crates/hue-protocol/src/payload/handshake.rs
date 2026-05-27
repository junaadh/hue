use crate::wire::ProtocolError;

pub const CAPS_ARTWORK: u16 = 1 << 0;
pub const CAPS_COMMANDS: u16 = 1 << 1;
pub const CAPS_DELTA: u16 = 1 << 2;

pub const RESERVED_CAPS_MASK: u16 = 0b1111_1111_1111_1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HandshakePayload {
    pub proto_version: u8,
    pub min_proto_version: u8,
    pub caps: u16,
    pub epoch: u32,
}

impl HandshakePayload {
    pub const LEN: usize = 8;

    pub fn encode(&self, out: &mut [u8]) -> Result<usize, ProtocolError> {
        if out.len() < Self::LEN {
            return Err(ProtocolError::InvalidLength);
        }

        if self.caps & RESERVED_CAPS_MASK != 0 {
            return Err(ProtocolError::ReservedNonZero);
        }

        out[0] = self.proto_version;
        out[1] = self.min_proto_version;

        out[2..4].copy_from_slice(&self.caps.to_le_bytes());

        out[4..8].copy_from_slice(&self.epoch.to_le_bytes());

        Ok(Self::LEN)
    }

    pub fn decode(payload: &[u8]) -> Result<Self, ProtocolError> {
        if payload.len() != Self::LEN {
            return Err(ProtocolError::InvalidLength);
        }

        let caps = u16::from_le_bytes([payload[2], payload[3]]);

        if caps & RESERVED_CAPS_MASK != 0 {
            return Err(ProtocolError::ReservedNonZero);
        }

        Ok(Self {
            proto_version: payload[0],
            min_proto_version: payload[1],
            caps,
            epoch: u32::from_le_bytes([
                payload[4], payload[5], payload[6], payload[7],
            ]),
        })
    }

    pub fn supports_artwork(&self) -> bool {
        self.caps & CAPS_ARTWORK != 0
    }

    pub fn supports_commands(&self) -> bool {
        self.caps & CAPS_COMMANDS != 0
    }

    pub fn supports_delta(&self) -> bool {
        self.caps & CAPS_DELTA != 0
    }
}
