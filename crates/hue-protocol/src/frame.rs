use crate::{
    Result,
    wire::{
        Crc, HEADER_LEN, MAGIC, MAX_PAYLOAD_SIZE, MessageType, ProtocolError,
        RESERVED_FLAG_MASK, VERSION_V0,
    },
};

#[derive(Debug, Clone, Copy)]
pub struct FrameHeader {
    pub version: u8,
    pub msg_type: MessageType,
    pub flags: u8,
    pub len: u16,
    pub seq: u16,
    pub crc: u16,
}

impl FrameHeader {
    pub fn parse(header: &[u8; HEADER_LEN]) -> Result<Self> {
        if header[0] != MAGIC[0] || header[1] != MAGIC[1] {
            return Err(ProtocolError::InvalidMagic);
        }

        let version = header[2];
        if version != VERSION_V0 {
            return Err(ProtocolError::InvalidVersion);
        }

        let raw_type = header[3];
        let msg_type = MessageType::from_u8(raw_type)?;

        let flags = header[4];
        if flags & RESERVED_FLAG_MASK != 0 || header[5] != 0 {
            return Err(ProtocolError::ReservedNonZero);
        }

        let len = u16::from_le_bytes([header[6], header[7]]);
        if len as usize > MAX_PAYLOAD_SIZE {
            return Err(ProtocolError::OversizedPayload);
        }

        let seq = u16::from_le_bytes([header[8], header[9]]);
        let crc = u16::from_le_bytes([header[10], header[11]]);

        Ok(Self {
            version,
            msg_type,
            flags,
            len,
            seq,
            crc,
        })
    }

    pub fn write_without_crc(
        out: &mut [u8],
        msg_type: MessageType,
        flags: u8,
        payload_len: usize,
        seq: u16,
    ) -> Result<()> {
        if out.len() < HEADER_LEN {
            return Err(ProtocolError::InvalidLength);
        }

        if payload_len > MAX_PAYLOAD_SIZE {
            return Err(ProtocolError::OversizedPayload);
        }

        if flags & RESERVED_FLAG_MASK != 0 {
            return Err(ProtocolError::ReservedNonZero);
        }

        out[0] = MAGIC[0];
        out[1] = MAGIC[1];
        out[2] = VERSION_V0;
        out[3] = msg_type as u8;
        out[4] = flags;
        out[5] = 0;

        let len = payload_len as u16;
        out[6..8].copy_from_slice(&len.to_le_bytes());
        out[8..10].copy_from_slice(&seq.to_le_bytes());

        out[10] = 0;
        out[11] = 0;

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FrameView<'a> {
    pub header: FrameHeader,
    pub payload: &'a [u8],
}

pub fn compute_frame_crc(header: &[u8; HEADER_LEN], payload: &[u8]) -> Crc {
    let mut crc = Crc::INIT;
    crc.feed(&header[0..10]);
    crc.feed(&[0, 0]);
    crc.feed(payload);
    crc
}

pub fn validate_frame_crc(
    header: &[u8; HEADER_LEN],
    payload: &[u8],
) -> Result<()> {
    let expected = Crc::new(u16::from_le_bytes([header[10], header[11]]));
    let actual = compute_frame_crc(header, payload);

    if expected != actual {
        return Err(ProtocolError::CrcMismatch);
    }

    Ok(())
}

pub fn encode_frame(
    out: &mut [u8],
    msg_type: MessageType,
    flags: u8,
    seq: u16,
    payload: &[u8],
) -> Result<usize> {
    let total_len = HEADER_LEN + payload.len();

    if out.len() < total_len {
        return Err(ProtocolError::InvalidLength);
    }

    FrameHeader::write_without_crc(out, msg_type, flags, payload.len(), seq)?;

    out[HEADER_LEN..total_len].copy_from_slice(payload);

    let mut header = [0u8; HEADER_LEN];
    header.copy_from_slice(&out[..HEADER_LEN]);

    let crc = compute_frame_crc(&header, payload);
    out[10..12].copy_from_slice(&crc.raw().to_le_bytes());

    Ok(total_len)
}
