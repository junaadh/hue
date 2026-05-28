use crate::{Result, wire::ProtocolError};

pub const ARTWORK_FORMAT_RGB565: u8 = 0x00;

pub const ARTWORK_ABORT_TRACK_CHANGED: u8 = 0x00;
pub const ARTWORK_ABORT_ENCODER_ERROR: u8 = 0x01;
pub const ARTWORK_ABORT_SENDER_REQUEST: u8 = 0x02;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArtworkBeginPayload {
    pub artwork_id: u32,
    pub width: u16,
    pub height: u16,
    pub total_bytes: u32,
    pub format: u8,
}

impl ArtworkBeginPayload {
    pub const LEN: usize = 24;

    pub fn encode(&self, out: &mut [u8]) -> Result<usize> {
        if out.len() < Self::LEN {
            return Err(ProtocolError::InvalidLength);
        }

        validate_dimensions(self.width, self.height, self.total_bytes)?;
        validate_format(self.format)?;

        out[..Self::LEN].fill(0);
        out[0..4].copy_from_slice(&self.artwork_id.to_le_bytes());
        out[4..6].copy_from_slice(&self.width.to_le_bytes());
        out[6..8].copy_from_slice(&self.height.to_le_bytes());
        out[8..12].copy_from_slice(&self.total_bytes.to_le_bytes());
        out[12] = self.format;

        Ok(Self::LEN)
    }

    pub fn decode(payload: &[u8]) -> Result<Self> {
        if payload.len() != Self::LEN {
            return Err(ProtocolError::InvalidLength);
        }

        if payload[13..24].iter().any(|&byte| byte != 0) {
            return Err(ProtocolError::ReservedNonZero);
        }

        let out = Self {
            artwork_id: u32::from_le_bytes([
                payload[0], payload[1], payload[2], payload[3],
            ]),
            width: u16::from_le_bytes([payload[4], payload[5]]),
            height: u16::from_le_bytes([payload[6], payload[7]]),
            total_bytes: u32::from_le_bytes([
                payload[8],
                payload[9],
                payload[10],
                payload[11],
            ]),
            format: payload[12],
        };

        validate_dimensions(out.width, out.height, out.total_bytes)?;
        validate_format(out.format)?;

        Ok(out)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArtworkChunkPayload<'a> {
    pub artwork_id: u32,
    pub chunk_index: u16,
    pub data: &'a [u8],
}

impl<'a> ArtworkChunkPayload<'a> {
    pub const HEADER_LEN: usize = 8;

    pub fn encode(&self, out: &mut [u8]) -> Result<usize> {
        validate_chunk_data_len(self.data.len())?;

        let len = Self::HEADER_LEN + self.data.len();
        if out.len() < len {
            return Err(ProtocolError::InvalidLength);
        }

        out[0..4].copy_from_slice(&self.artwork_id.to_le_bytes());
        out[4..6].copy_from_slice(&self.chunk_index.to_le_bytes());
        out[6..8].copy_from_slice(&(self.data.len() as u16).to_le_bytes());
        out[Self::HEADER_LEN..len].copy_from_slice(self.data);

        Ok(len)
    }

    pub fn decode(payload: &'a [u8]) -> Result<Self> {
        if payload.len() < Self::HEADER_LEN {
            return Err(ProtocolError::InvalidLength);
        }

        let data_len = u16::from_le_bytes([payload[6], payload[7]]) as usize;
        validate_chunk_data_len(data_len)?;

        let expected_len = Self::HEADER_LEN
            .checked_add(data_len)
            .ok_or(ProtocolError::InvalidLength)?;
        if payload.len() != expected_len {
            return Err(ProtocolError::InvalidLength);
        }

        Ok(Self {
            artwork_id: u32::from_le_bytes([
                payload[0], payload[1], payload[2], payload[3],
            ]),
            chunk_index: u16::from_le_bytes([payload[4], payload[5]]),
            data: &payload[Self::HEADER_LEN..],
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArtworkEndPayload {
    pub artwork_id: u32,
    pub checksum: u32,
}

impl ArtworkEndPayload {
    pub const LEN: usize = 8;

    pub fn encode(&self, out: &mut [u8]) -> Result<usize> {
        if out.len() < Self::LEN {
            return Err(ProtocolError::InvalidLength);
        }

        out[0..4].copy_from_slice(&self.artwork_id.to_le_bytes());
        out[4..8].copy_from_slice(&self.checksum.to_le_bytes());

        Ok(Self::LEN)
    }

    pub fn decode(payload: &[u8]) -> Result<Self> {
        if payload.len() != Self::LEN {
            return Err(ProtocolError::InvalidLength);
        }

        Ok(Self {
            artwork_id: u32::from_le_bytes([
                payload[0], payload[1], payload[2], payload[3],
            ]),
            checksum: u32::from_le_bytes([
                payload[4], payload[5], payload[6], payload[7],
            ]),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArtworkAbortPayload {
    pub artwork_id: u32,
    pub reason: u8,
}

impl ArtworkAbortPayload {
    pub const LEN: usize = 8;

    pub fn encode(&self, out: &mut [u8]) -> Result<usize> {
        if out.len() < Self::LEN {
            return Err(ProtocolError::InvalidLength);
        }

        validate_abort_reason(self.reason)?;

        out[..Self::LEN].fill(0);
        out[0..4].copy_from_slice(&self.artwork_id.to_le_bytes());
        out[4] = self.reason;

        Ok(Self::LEN)
    }

    pub fn decode(payload: &[u8]) -> Result<Self> {
        if payload.len() != Self::LEN {
            return Err(ProtocolError::InvalidLength);
        }

        if payload[5] != 0 || payload[6] != 0 || payload[7] != 0 {
            return Err(ProtocolError::ReservedNonZero);
        }

        let out = Self {
            artwork_id: u32::from_le_bytes([
                payload[0], payload[1], payload[2], payload[3],
            ]),
            reason: payload[4],
        };

        validate_abort_reason(out.reason)?;

        Ok(out)
    }
}

fn validate_dimensions(
    width: u16,
    height: u16,
    total_bytes: u32,
) -> Result<()> {
    if width == 0 || height == 0 {
        return Err(ProtocolError::InvalidLength);
    }

    let expected = u32::from(width)
        .checked_mul(u32::from(height))
        .and_then(|pixels| pixels.checked_mul(2))
        .ok_or(ProtocolError::InvalidLength)?;

    if total_bytes != expected {
        return Err(ProtocolError::InvalidLength);
    }

    Ok(())
}

fn validate_format(format: u8) -> Result<()> {
    if format != ARTWORK_FORMAT_RGB565 {
        return Err(ProtocolError::InvalidLength);
    }

    Ok(())
}

fn validate_chunk_data_len(data_len: usize) -> Result<()> {
    if data_len > u16::MAX as usize {
        return Err(ProtocolError::InvalidLength);
    }

    if !data_len.is_multiple_of(2) {
        return Err(ProtocolError::InvalidAlignment);
    }

    Ok(())
}

fn validate_abort_reason(reason: u8) -> Result<()> {
    match reason {
        ARTWORK_ABORT_TRACK_CHANGED
        | ARTWORK_ABORT_ENCODER_ERROR
        | ARTWORK_ABORT_SENDER_REQUEST => Ok(()),
        _ => Err(ProtocolError::InvalidLength),
    }
}
