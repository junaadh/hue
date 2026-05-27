use crate::{
    Result,
    delta::diff_state,
    frame::{FrameHeader, compute_frame_crc},
    payload::{
        AckPayload, ArtworkAbortPayload, ArtworkBeginPayload,
        ArtworkChunkPayload, ArtworkEndPayload, HandshakePayload, NackPayload,
        encode_state_delta,
    },
    wire::{FLAG_IS_RESP, HEADER_LEN, MessageType, ProtocolError},
};
use hue_core::state::UiState;

pub struct FrameBuilder;

impl FrameBuilder {
    pub fn ping(out: &mut [u8], seq: u16) -> Result<usize> {
        Self::empty(out, MessageType::Ping, seq)
    }

    pub fn pong(out: &mut [u8], seq: u16) -> Result<usize> {
        Self::empty(out, MessageType::Pong, seq)
    }

    pub fn resync(out: &mut [u8], seq: u16) -> Result<usize> {
        Self::empty(out, MessageType::Resync, seq)
    }

    pub fn state_delta(
        out: &mut [u8],
        seq: u16,
        old_state: &UiState,
        new_state: &UiState,
    ) -> Result<usize> {
        if out.len() < HEADER_LEN {
            return Err(ProtocolError::InvalidLength);
        }

        let field_mask = diff_state(old_state, new_state);
        let payload_len = {
            let payload_out = &mut out[HEADER_LEN..];
            encode_state_delta(new_state, field_mask, payload_out)?
        };

        Self::finish_frame(out, MessageType::StateDelta, 0, seq, payload_len)
    }

    pub fn ack(out: &mut [u8], seq: u16, acked_seq: u16) -> Result<usize> {
        if out.len() < HEADER_LEN {
            return Err(ProtocolError::InvalidLength);
        }

        let payload_len =
            AckPayload { acked_seq }.encode(&mut out[HEADER_LEN..])?;

        Self::finish_frame(
            out,
            MessageType::Ack,
            FLAG_IS_RESP,
            seq,
            payload_len,
        )
    }

    pub fn handshake(
        out: &mut [u8],
        seq: u16,
        payload: HandshakePayload,
    ) -> Result<usize> {
        if out.len() < HEADER_LEN {
            return Err(ProtocolError::InvalidLength);
        }

        let payload_len = payload.encode(&mut out[HEADER_LEN..])?;

        Self::finish_frame(out, MessageType::Handshake, 0, seq, payload_len)
    }

    pub fn artwork_begin(
        out: &mut [u8],
        seq: u16,
        payload: ArtworkBeginPayload,
    ) -> Result<usize> {
        if out.len() < HEADER_LEN {
            return Err(ProtocolError::InvalidLength);
        }

        let payload_len = payload.encode(&mut out[HEADER_LEN..])?;

        Self::finish_frame(out, MessageType::ArtworkBegin, 0, seq, payload_len)
    }

    pub fn artwork_chunk(
        out: &mut [u8],
        seq: u16,
        payload: ArtworkChunkPayload<'_>,
    ) -> Result<usize> {
        if out.len() < HEADER_LEN {
            return Err(ProtocolError::InvalidLength);
        }

        let payload_len = payload.encode(&mut out[HEADER_LEN..])?;

        Self::finish_frame(out, MessageType::ArtworkChunk, 0, seq, payload_len)
    }

    pub fn artwork_end(
        out: &mut [u8],
        seq: u16,
        payload: ArtworkEndPayload,
    ) -> Result<usize> {
        if out.len() < HEADER_LEN {
            return Err(ProtocolError::InvalidLength);
        }

        let payload_len = payload.encode(&mut out[HEADER_LEN..])?;

        Self::finish_frame(out, MessageType::ArtworkEnd, 0, seq, payload_len)
    }

    pub fn artwork_abort(
        out: &mut [u8],
        seq: u16,
        payload: ArtworkAbortPayload,
    ) -> Result<usize> {
        if out.len() < HEADER_LEN {
            return Err(ProtocolError::InvalidLength);
        }

        let payload_len = payload.encode(&mut out[HEADER_LEN..])?;

        Self::finish_frame(out, MessageType::ArtworkAbort, 0, seq, payload_len)
    }

    pub fn nack(
        out: &mut [u8],
        seq: u16,
        nacked_seq: u16,
        error: ProtocolError,
        context: MessageType,
    ) -> Result<usize> {
        if out.len() < HEADER_LEN {
            return Err(ProtocolError::InvalidLength);
        }

        let payload_len = NackPayload {
            nacked_seq,
            error_code: error as u8,
            context_type: context as u8,
        }
        .encode(&mut out[HEADER_LEN..])?;

        Self::finish_frame(
            out,
            MessageType::Nack,
            FLAG_IS_RESP,
            seq,
            payload_len,
        )
    }

    fn empty(out: &mut [u8], msg_type: MessageType, seq: u16) -> Result<usize> {
        Self::finish_frame(out, msg_type, 0, seq, 0)
    }

    fn finish_frame(
        out: &mut [u8],
        msg_type: MessageType,
        flags: u8,
        seq: u16,
        payload_len: usize,
    ) -> Result<usize> {
        let total_len = HEADER_LEN + payload_len;

        if out.len() < total_len {
            return Err(ProtocolError::InvalidLength);
        }

        FrameHeader::write_without_crc(
            &mut out[..HEADER_LEN],
            msg_type,
            flags,
            payload_len,
            seq,
        )?;

        let mut header = [0u8; HEADER_LEN];
        header.copy_from_slice(&out[..HEADER_LEN]);

        let crc = compute_frame_crc(&header, &out[HEADER_LEN..total_len]);
        out[10..12].copy_from_slice(&crc.raw().to_le_bytes());

        Ok(total_len)
    }
}
