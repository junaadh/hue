use crate::{
    frame::{FrameHeader, validate_frame_crc},
    wire::{HEADER_LEN, MAGIC, MAX_PAYLOAD_SIZE, ProtocolError},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParseState {
    Hunt,
    Magic2,
    Header,
    Payload,
    Discard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedResult {
    NeedMore,
    FrameReady,
    Error(ProtocolError),
}

pub struct FrameParser {
    state: ParseState,
    header_buf: [u8; HEADER_LEN],
    header_pos: usize,
    payload_buf: [u8; MAX_PAYLOAD_SIZE],
    payload_pos: usize,
    payload_len: usize,
    discard_count: usize,
    ready_header: Option<FrameHeader>,
}

impl FrameParser {
    pub const fn new() -> Self {
        Self {
            state: ParseState::Hunt,
            header_buf: [0; HEADER_LEN],
            header_pos: 0,
            payload_buf: [0; MAX_PAYLOAD_SIZE],
            payload_pos: 0,
            payload_len: 0,
            discard_count: 0,
            ready_header: None,
        }
    }

    pub fn feed_byte(&mut self, byte: u8) -> FeedResult {
        match self.state {
            ParseState::Hunt => {
                if byte == MAGIC[0] {
                    self.header_buf[0] = byte;
                    self.header_pos = 1;
                    self.state = ParseState::Magic2;
                }
                FeedResult::NeedMore
            }

            ParseState::Magic2 => {
                if byte == MAGIC[1] {
                    self.header_buf[1] = byte;
                    self.header_pos = 2;
                    self.state = ParseState::Header;
                } else if byte == MAGIC[0] {
                    self.header_buf[0] = byte;
                    self.header_pos = 1;
                    self.state = ParseState::Magic2;
                } else {
                    self.reset();
                }

                FeedResult::NeedMore
            }

            ParseState::Header => {
                self.header_buf[self.header_pos] = byte;
                self.header_pos += 1;

                if self.header_pos < HEADER_LEN {
                    return FeedResult::NeedMore;
                }

                let payload_len = u16::from_le_bytes([
                    self.header_buf[6],
                    self.header_buf[7],
                ]) as usize;

                if payload_len > MAX_PAYLOAD_SIZE {
                    self.discard_count = payload_len;
                    self.state = ParseState::Discard;
                    return FeedResult::NeedMore;
                }

                let header = match FrameHeader::parse(&self.header_buf) {
                    Ok(header) => header,
                    Err(err) => {
                        self.reset();
                        return FeedResult::Error(err);
                    }
                };

                self.payload_len = header.len as usize;
                self.payload_pos = 0;

                if self.payload_len > MAX_PAYLOAD_SIZE {
                    self.discard_count = self.payload_len;
                    self.state = ParseState::Discard;
                    return FeedResult::NeedMore;
                }

                if self.payload_len == 0 {
                    return self.finish_frame(header);
                }

                self.state = ParseState::Payload;
                FeedResult::NeedMore
            }

            ParseState::Payload => {
                self.payload_buf[self.payload_pos] = byte;
                self.payload_pos += 1;

                if self.payload_pos < self.payload_len {
                    return FeedResult::NeedMore;
                }

                let header = match FrameHeader::parse(&self.header_buf) {
                    Ok(header) => header,
                    Err(err) => {
                        self.reset();
                        return FeedResult::Error(err);
                    }
                };

                self.finish_frame(header)
            }

            ParseState::Discard => {
                if self.discard_count > 0 {
                    self.discard_count -= 1;
                }

                if self.discard_count == 0 {
                    self.reset();
                    return FeedResult::Error(ProtocolError::OversizedPayload);
                }

                FeedResult::NeedMore
            }
        }
    }

    pub fn take_frame(&mut self) -> Option<ParsedFrame<'_>> {
        let header = self.ready_header.take()?;
        let len = header.len as usize;

        Some(ParsedFrame {
            header,
            payload: &self.payload_buf[..len],
        })
    }

    fn finish_frame(&mut self, header: FrameHeader) -> FeedResult {
        let payload = &self.payload_buf[..self.payload_len];

        if let Err(err) = validate_frame_crc(&self.header_buf, payload) {
            self.reset();
            return FeedResult::Error(err);
        }

        self.ready_header = Some(header);
        self.state = ParseState::Hunt;
        self.header_pos = 0;
        self.payload_pos = 0;
        self.payload_len = 0;

        FeedResult::FrameReady
    }

    fn reset(&mut self) {
        self.state = ParseState::Hunt;
        self.header_pos = 0;
        self.payload_pos = 0;
        self.payload_len = 0;
        self.discard_count = 0;
        self.ready_header = None;
    }
}

impl Default for FrameParser {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ParsedFrame<'a> {
    pub header: FrameHeader,
    pub payload: &'a [u8],
}
