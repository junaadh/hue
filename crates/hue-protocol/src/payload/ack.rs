use crate::{Result, wire::ProtocolError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AckPayload {
    pub acked_seq: u16,
}

impl AckPayload {
    pub const LEN: usize = 4;

    pub fn encode(&self, out: &mut [u8]) -> Result<usize> {
        if out.len() < Self::LEN {
            return Err(ProtocolError::InvalidLength);
        }

        out[0..2].copy_from_slice(&self.acked_seq.to_le_bytes());

        // padding
        out[2] = 0;
        out[3] = 0;

        Ok(Self::LEN)
    }

    pub fn decode(payload: &[u8]) -> Result<Self> {
        if payload.len() != Self::LEN {
            return Err(ProtocolError::InvalidLength);
        }

        Ok(Self {
            acked_seq: u16::from_le_bytes([payload[0], payload[1]]),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NackPayload {
    pub nacked_seq: u16,
    pub error_code: u8,
    pub context_type: u8,
}

impl NackPayload {
    pub const LEN: usize = 4;

    pub fn encode(&self, out: &mut [u8]) -> Result<usize> {
        if out.len() < Self::LEN {
            return Err(ProtocolError::InvalidLength);
        }

        out[0..2].copy_from_slice(&self.nacked_seq.to_le_bytes());
        out[2] = self.error_code;
        out[3] = self.context_type;

        Ok(Self::LEN)
    }

    pub fn decode(payload: &[u8]) -> Result<Self> {
        if payload.len() != Self::LEN {
            return Err(ProtocolError::InvalidLength);
        }

        Ok(Self {
            nacked_seq: u16::from_le_bytes([payload[0], payload[1]]),
            error_code: payload[2],
            context_type: payload[3],
        })
    }
}
