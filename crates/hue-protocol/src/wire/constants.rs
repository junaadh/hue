pub const MAGIC: [u8; 2] = *b"HU";

pub const VERSION_V0: u8 = 0;
pub const HEADER_LEN: usize = 12;
pub const MAX_PAYLOAD_SIZE: usize = 512;

pub const FLAG_ACK_REQ: u8 = 1 << 1;
pub const FLAG_IS_RESP: u8 = 1 << 2;
pub const FLAG_EPOCH: u8 = 1 << 3;
pub const FLAG_FRAG: u8 = 1 << 4;
pub const FLAG_LAST_FRAG: u8 = 1 << 5;

pub const RESERVED_FLAG_MASK: u8 = 0b1100_0001;
