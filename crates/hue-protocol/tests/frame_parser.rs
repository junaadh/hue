use hue_protocol::{
    frame::encode_frame,
    parser::{FeedResult, FrameParser},
    wire::{HEADER_LEN, MessageType, ProtocolError},
};

#[test]
fn encodes_and_parses_ping() {
    let mut buf = [0u8; 64];

    let len = encode_frame(&mut buf, MessageType::Ping, 0, 42, &[]).unwrap();

    assert_eq!(len, HEADER_LEN);

    let mut parser = FrameParser::new();

    for &byte in &buf[..len - 1] {
        assert_eq!(parser.feed_byte(byte), FeedResult::NeedMore);
    }

    assert_eq!(parser.feed_byte(buf[len - 1]), FeedResult::FrameReady);

    let frame = parser.take_frame().unwrap();
    assert_eq!(frame.header.msg_type, MessageType::Ping);
    assert_eq!(frame.header.seq, 42);
    assert_eq!(frame.payload, &[]);
}

#[test]
fn encodes_and_parses_payload_frame() {
    let payload = [1u8, 2, 3, 4];
    let mut buf = [0u8; 64];

    let len =
        encode_frame(&mut buf, MessageType::Pong, 0, 7, &payload).unwrap();

    let mut parser = FrameParser::new();

    let mut result = FeedResult::NeedMore;
    for &byte in &buf[..len] {
        result = parser.feed_byte(byte);
    }

    assert_eq!(result, FeedResult::FrameReady);

    let frame = parser.take_frame().unwrap();
    assert_eq!(frame.header.msg_type, MessageType::Pong);
    assert_eq!(frame.header.seq, 7);
    assert_eq!(frame.payload, payload);
}

#[test]
fn crc_mismatch_is_rejected() {
    let payload = [1u8, 2, 3, 4];
    let mut buf = [0u8; 64];

    let len =
        encode_frame(&mut buf, MessageType::Pong, 0, 7, &payload).unwrap();

    buf[HEADER_LEN + 1] ^= 0x01;

    let mut parser = FrameParser::new();

    let mut result = FeedResult::NeedMore;
    for &byte in &buf[..len] {
        result = parser.feed_byte(byte);
    }

    assert_eq!(result, FeedResult::Error(ProtocolError::CrcMismatch));
}

#[test]
fn reserved_flags_are_rejected() {
    let payload = [];
    let mut buf = [0u8; 64];

    let len =
        encode_frame(&mut buf, MessageType::Ping, 0, 1, &payload).unwrap();

    buf[4] = 0b1000_0000;

    // Need to recompute CRC? No. This should fail before CRC at header parse.
    let mut parser = FrameParser::new();

    let mut result = FeedResult::NeedMore;
    for &byte in &buf[..len] {
        result = parser.feed_byte(byte);
    }

    assert_eq!(result, FeedResult::Error(ProtocolError::ReservedNonZero));
}
