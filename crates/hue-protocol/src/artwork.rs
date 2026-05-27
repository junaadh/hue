use crate::{
    Result,
    payload::{
        ARTWORK_ABORT_ENCODER_ERROR, ARTWORK_ABORT_SENDER_REQUEST,
        ARTWORK_ABORT_TRACK_CHANGED, ArtworkAbortPayload, ArtworkBeginPayload,
        ArtworkChunkPayload, ArtworkEndPayload,
    },
    wire::ProtocolError,
};
use hue_core::hash::Fnv32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtworkRxStatus {
    Began {
        artwork_id: u32,
        total_bytes: u32,
    },
    ChunkAccepted {
        artwork_id: u32,
        chunk_index: u16,
        received_bytes: u32,
        total_bytes: u32,
    },
    Complete {
        artwork_id: u32,
        total_chunks: u16,
        total_bytes: u32,
        checksum: u32,
    },
    Aborted {
        artwork_id: u32,
        reason: u8,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArtworkRx {
    active: bool,
    artwork_id: u32,
    expected_chunk_index: u16,
    total_chunks: u16,
    total_bytes: u32,
    received_bytes: u32,
    checksum: Fnv32,
}

impl ArtworkRx {
    pub const fn new() -> Self {
        Self {
            active: false,
            artwork_id: 0,
            expected_chunk_index: 0,
            total_chunks: 0,
            total_bytes: 0,
            received_bytes: 0,
            checksum: Fnv32::new(),
        }
    }

    pub const fn is_active(&self) -> bool {
        self.active
    }

    pub const fn artwork_id(&self) -> Option<u32> {
        if self.active {
            Some(self.artwork_id)
        } else {
            None
        }
    }

    pub const fn expected_chunk_index(&self) -> u16 {
        self.expected_chunk_index
    }

    pub const fn total_chunks(&self) -> u16 {
        self.total_chunks
    }

    pub const fn total_bytes(&self) -> u32 {
        self.total_bytes
    }

    pub const fn received_bytes(&self) -> u32 {
        self.received_bytes
    }

    pub const fn checksum(&self) -> u32 {
        self.checksum.finish()
    }

    pub fn begin(
        &mut self,
        payload: ArtworkBeginPayload,
    ) -> Result<ArtworkRxStatus> {
        if self.active {
            return Err(ProtocolError::InvalidLength);
        }

        self.active = true;
        self.artwork_id = payload.artwork_id;
        self.expected_chunk_index = 0;
        self.total_chunks = 0;
        self.total_bytes = payload.total_bytes;
        self.received_bytes = 0;
        self.checksum = Fnv32::new();

        Ok(ArtworkRxStatus::Began {
            artwork_id: payload.artwork_id,
            total_bytes: payload.total_bytes,
        })
    }

    pub fn chunk(
        &mut self,
        payload: ArtworkChunkPayload<'_>,
    ) -> Result<ArtworkRxStatus> {
        self.ensure_active(payload.artwork_id)?;

        if payload.chunk_index != self.expected_chunk_index {
            return Err(ProtocolError::ChunkIndexMismatch);
        }

        if payload.data.len() > u16::MAX as usize {
            return Err(ProtocolError::InvalidLength);
        }

        if payload.data.len() % 2 != 0 {
            return Err(ProtocolError::InvalidAlignment);
        }

        let received_bytes = self
            .received_bytes
            .checked_add(payload.data.len() as u32)
            .ok_or(ProtocolError::InvalidLength)?;

        if received_bytes > self.total_bytes {
            return Err(ProtocolError::InvalidLength);
        }

        let next_chunk = self
            .expected_chunk_index
            .checked_add(1)
            .ok_or(ProtocolError::InvalidLength)?;

        self.checksum.feed_bytes(payload.data);
        self.received_bytes = received_bytes;
        self.expected_chunk_index = next_chunk;
        self.total_chunks = next_chunk;

        Ok(ArtworkRxStatus::ChunkAccepted {
            artwork_id: payload.artwork_id,
            chunk_index: payload.chunk_index,
            received_bytes,
            total_bytes: self.total_bytes,
        })
    }

    pub fn end(
        &mut self,
        payload: ArtworkEndPayload,
    ) -> Result<ArtworkRxStatus> {
        self.ensure_active(payload.artwork_id)?;

        if self.received_bytes != self.total_bytes {
            return Err(ProtocolError::InvalidLength);
        }

        let checksum = self.checksum.finish();
        if payload.checksum != checksum {
            return Err(ProtocolError::ChecksumMismatch);
        }

        let status = ArtworkRxStatus::Complete {
            artwork_id: payload.artwork_id,
            total_chunks: self.total_chunks,
            total_bytes: self.total_bytes,
            checksum,
        };

        self.reset();

        Ok(status)
    }

    pub fn abort(
        &mut self,
        payload: ArtworkAbortPayload,
    ) -> Result<ArtworkRxStatus> {
        self.ensure_active(payload.artwork_id)?;

        match payload.reason {
            ARTWORK_ABORT_TRACK_CHANGED
            | ARTWORK_ABORT_ENCODER_ERROR
            | ARTWORK_ABORT_SENDER_REQUEST => {}
            _ => return Err(ProtocolError::InvalidLength),
        }

        let status = ArtworkRxStatus::Aborted {
            artwork_id: payload.artwork_id,
            reason: payload.reason,
        };

        self.reset();

        Ok(status)
    }

    fn ensure_active(&self, artwork_id: u32) -> Result<()> {
        if !self.active {
            return Err(ProtocolError::InvalidLength);
        }

        if artwork_id != self.artwork_id {
            return Err(ProtocolError::ArtworkIdMismatch);
        }

        Ok(())
    }

    fn reset(&mut self) {
        *self = Self::new();
    }
}

impl Default for ArtworkRx {
    fn default() -> Self {
        Self::new()
    }
}
