//! Hue protocol v0 wire format.
//!
//! Every frame starts with a 12-byte little-endian header:
//!
//! ```text
//! +0   u8[2] magic      "HU"
//! +2   u8    version    0
//! +3   u8    msg_type   see MessageType
//! +4   u8    flags
//! +5   u8    reserved   must be 0
//! +6   u16   len        payload length in bytes
//! +8   u16   seq        wrapping sequence number
//! +10  u16   crc        CRC-CCITT over header with crc zeroed + payload
//! ```
//!
//! CRC is mandatory for every frame, including empty-payload control frames.
//! Receivers must reject `CrcMismatch` and continue parsing subsequent bytes.
//!
//! Valid flag bits are `FLAG_ACK_REQ`, `FLAG_IS_RESP`, `FLAG_EPOCH`,
//! `FLAG_FRAG`, and `FLAG_LAST_FRAG`. Reserved flag bits and header byte 5
//! must be zero. Payload-specific reserved bytes must also be zero.
//!
//! `StateDelta` payloads have a 32-byte fixed section followed by selected
//! variable-length UTF-8 strings:
//!
//! ```text
//! +0   u16 field_mask
//! +2   u32 track_id
//! +6   u64 position_ms
//! +14  u64 duration_ms
//! +22  u8  playing
//! +23  u8  title_len
//! +24  u8  artist_len
//! +25  u8  album_len
//! +26  u8  app_name_len
//! +27  u8  reserved
//! +28  u32 artwork_id
//! +32  variable string bytes in title/artist/album/app_name order
//! ```
//!
//! Decoding is atomic: invalid field masks, lengths, reserved bytes, or UTF-8
//! errors must not partially mutate the output state.
//!
//! Artwork streams are validated as protocol state only. The valid order is
//! `ArtworkBegin -> ArtworkChunk(0) -> ArtworkChunk(1) -> ... -> ArtworkEnd`.
//! The receiver tracks the active artwork id, expected chunk index, total
//! bytes, received bytes, chunk count, and rolling FNV-1a checksum. `End`
//! succeeds only after all declared bytes arrive and the checksum matches.
//! `ArtworkAbort` cancels the active stream.
//!
//! `FrameParser` is a streaming parser for USB/UART-style byte streams. It
//! hunts for `HU`, handles split frames and back-to-back frames, drains
//! oversized payloads before resyncing, validates CRC before exposing frames,
//! and never allocates.
//!
//! The protocol crate is `no_std`. Core parser/builder/payload paths operate
//! in caller-provided buffers and fixed-size internal storage; dynamic
//! allocation is not required by the wire protocol implementation.
#![no_std]

pub mod artwork;
pub mod delta;
pub mod encoder;
pub mod frame;
pub mod parser;
pub mod payload;
pub mod seq;
pub mod wire;

pub mod builder;

pub type Result<T> = core::result::Result<T, wire::ProtocolError>;

pub use artwork::{ArtworkRx, ArtworkRxStatus};
pub use seq::{SeqStatus, SeqTracker, seq_distance};
