use hue_protocol::{
    frame::encode_frame,
    parser::{FeedResult, FrameParser},
    wire::MessageType,
};

#[test]
fn parser_recovers_after_crc_failure() {
    let mut good1 = [0u8; 64];
    let mut bad = [0u8; 64];
    let mut good2 = [0u8; 64];

    let len1 = encode_frame(&mut good1, MessageType::Ping, 0, 1, &[]).unwrap();

    let bad_len =
        encode_frame(&mut bad, MessageType::Pong, 0, 2, &[1, 2, 3, 4]).unwrap();

    let len2 = encode_frame(&mut good2, MessageType::Ack, 0, 3, &[9, 9, 9, 9])
        .unwrap();

    bad[15] ^= 0x55;

    let mut stream = [0u8; 256];
    let mut pos = 0;

    stream[pos..pos + len1].copy_from_slice(&good1[..len1]);
    pos += len1;

    stream[pos..pos + bad_len].copy_from_slice(&bad[..bad_len]);
    pos += bad_len;

    stream[pos..pos + len2].copy_from_slice(&good2[..len2]);
    pos += len2;

    let mut parser = FrameParser::new();

    let mut frames = 0;

    for &byte in &stream[..pos] {
        match parser.feed_byte(byte) {
            FeedResult::FrameReady => {
                let frame = parser.take_frame().unwrap();

                match frames {
                    0 => {
                        assert_eq!(frame.header.msg_type, MessageType::Ping);
                    }
                    1 => {
                        assert_eq!(frame.header.msg_type, MessageType::Ack);
                    }
                    _ => panic!("too many frames"),
                }

                frames += 1;
            }

            FeedResult::Error(_) => {}

            FeedResult::NeedMore => {}
        }
    }

    assert_eq!(frames, 2);
}
