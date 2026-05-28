use hue_core::{hstr::HStr, state::UiState};
use hue_protocol::{
    frame::{encode_frame, validate_frame_crc},
    parser::{FeedResult, FrameParser},
    payload::{
        FIELD_ALBUM, FIELD_APP_NAME, FIELD_ARTIST, FIELD_ARTWORK_ID,
        FIELD_DURATION, FIELD_PLAYING, FIELD_POSITION, FIELD_TITLE,
        FIELD_TRACK_ID, STATE_DELTA_FIXED_LEN, VALID_STATE_FIELDS,
        decode_state_delta, encode_state_delta,
    },
    seq_distance,
    wire::{HEADER_LEN, MAGIC, MAX_PAYLOAD_SIZE, MessageType, ProtocolError},
};

#[derive(Debug, PartialEq, Eq)]
struct CapturedFrame {
    msg_type: MessageType,
    flags: u8,
    len: u16,
    seq: u16,
    payload: Vec<u8>,
}

fn make_frame(
    msg_type: MessageType,
    flags: u8,
    seq: u16,
    payload: &[u8],
) -> Vec<u8> {
    let mut frame = vec![0; HEADER_LEN + payload.len()];
    let len = encode_frame(&mut frame, msg_type, flags, seq, payload).unwrap();
    frame.truncate(len);
    frame
}

fn capture_ready_frame(parser: &mut FrameParser) -> CapturedFrame {
    let frame = parser.take_frame().unwrap();

    CapturedFrame {
        msg_type: frame.header.msg_type,
        flags: frame.header.flags,
        len: frame.header.len,
        seq: frame.header.seq,
        payload: frame.payload.to_vec(),
    }
}

fn feed_collect(
    parser: &mut FrameParser,
    bytes: &[u8],
) -> (Vec<CapturedFrame>, Vec<ProtocolError>) {
    let mut frames = Vec::new();
    let mut errors = Vec::new();

    for &byte in bytes {
        match parser.feed_byte(byte) {
            FeedResult::FrameReady => frames.push(capture_ready_frame(parser)),
            FeedResult::Error(err) => errors.push(err),
            FeedResult::NeedMore => {}
        }
    }

    (frames, errors)
}

fn make_oversized_frame(payload: &[u8], seq: u16) -> Vec<u8> {
    assert!(payload.len() > MAX_PAYLOAD_SIZE);

    let mut frame = vec![0; HEADER_LEN + payload.len()];
    frame[0..2].copy_from_slice(&MAGIC);
    frame[2] = 0;
    frame[3] = MessageType::Ping as u8;
    frame[4] = 0;
    frame[5] = 0;
    frame[6..8].copy_from_slice(&(payload.len() as u16).to_le_bytes());
    frame[8..10].copy_from_slice(&seq.to_le_bytes());
    frame[10..12].copy_from_slice(&0u16.to_le_bytes());
    frame[HEADER_LEN..].copy_from_slice(payload);

    frame
}

fn full_state() -> UiState {
    let mut state = UiState::new();
    state.track_id = 0x1234_5678;
    state.title.set("Café del Mar 🌊".as_bytes()).unwrap();
    state.artist.set("Björk".as_bytes()).unwrap();
    state.album.set("Vespertine".as_bytes()).unwrap();
    state.app_name.set("Music.app".as_bytes()).unwrap();
    state.position_ms = 12_345;
    state.duration_ms = 234_567;
    state.playing = true;
    state.artwork_id = 0x8765_4321;
    state
}

fn all_state_fields() -> u16 {
    FIELD_TRACK_ID
        | FIELD_TITLE
        | FIELD_ARTIST
        | FIELD_ALBUM
        | FIELD_APP_NAME
        | FIELD_POSITION
        | FIELD_DURATION
        | FIELD_PLAYING
        | FIELD_ARTWORK_ID
}

#[test]
fn parser_accepts_valid_frame() {
    let payload = [0x10, 0x20, 0x30, 0x40, 0x50];
    let frame = make_frame(MessageType::Pong, 0, 42, &payload);
    let mut parser = FrameParser::new();

    for &byte in &frame[..frame.len() - 1] {
        assert_eq!(parser.feed_byte(byte), FeedResult::NeedMore);
    }

    assert_eq!(
        parser.feed_byte(*frame.last().unwrap()),
        FeedResult::FrameReady
    );

    let parsed = parser.take_frame().unwrap();
    assert_eq!(parsed.header.version, 0);
    assert_eq!(parsed.header.msg_type, MessageType::Pong);
    assert_eq!(parsed.header.flags, 0);
    assert_eq!(parsed.header.len, payload.len() as u16);
    assert_eq!(parsed.header.seq, 42);
    assert_eq!(parsed.payload, payload);

    let mut header = [0u8; HEADER_LEN];
    header.copy_from_slice(&frame[..HEADER_LEN]);
    assert_eq!(validate_frame_crc(&header, parsed.payload), Ok(()));

    assert!(parser.take_frame().is_none());

    let next = make_frame(MessageType::Ping, 0, 43, &[]);
    let (frames, errors) = feed_collect(&mut parser, &next);
    assert!(errors.is_empty());
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].seq, 43);
}

#[test]
fn parser_accepts_back_to_back_frames() {
    let first_payload = [1, 2, 3];
    let second_payload = [4, 5, 6, 7];
    let first = make_frame(MessageType::Ping, 0, 1, &first_payload);
    let second = make_frame(MessageType::Pong, 0, 2, &second_payload);
    let stream = [first.as_slice(), second.as_slice()].concat();
    let mut parser = FrameParser::new();

    let (frames, errors) = feed_collect(&mut parser, &stream);

    assert!(errors.is_empty());
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0].msg_type, MessageType::Ping);
    assert_eq!(frames[0].seq, 1);
    assert_eq!(frames[0].payload, first_payload);
    assert_eq!(frames[1].msg_type, MessageType::Pong);
    assert_eq!(frames[1].seq, 2);
    assert_eq!(frames[1].payload, second_payload);
}

#[test]
fn parser_accepts_frame_split_across_multiple_feeds() {
    let payload = [9, 8, 7, 6, 5, 4, 3];
    let frame = make_frame(MessageType::StateDelta, 0, 77, &payload);
    let mut parser = FrameParser::new();
    let chunks = [&frame[..3], &frame[3..8], &frame[8..9], &frame[9..]];

    for chunk in &chunks[..chunks.len() - 1] {
        for &byte in *chunk {
            assert_eq!(parser.feed_byte(byte), FeedResult::NeedMore);
        }
    }

    let mut result = FeedResult::NeedMore;
    for &byte in chunks.last().unwrap().iter() {
        result = parser.feed_byte(byte);
    }

    assert_eq!(result, FeedResult::FrameReady);
    let parsed = capture_ready_frame(&mut parser);
    assert_eq!(parsed.msg_type, MessageType::StateDelta);
    assert_eq!(parsed.seq, 77);
    assert_eq!(parsed.payload, payload);
}

#[test]
fn parser_skips_garbage_before_magic() {
    let frame = make_frame(MessageType::Ping, 0, 9, &[]);
    let mut stream = vec![13, 99, 0xff];
    stream.extend_from_slice(&frame);
    let mut parser = FrameParser::new();

    let (frames, errors) = feed_collect(&mut parser, &stream);

    assert!(errors.is_empty());
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].msg_type, MessageType::Ping);
    assert_eq!(frames[0].seq, 9);
}

#[test]
fn parser_handles_hhu_edge_case() {
    let frame = make_frame(MessageType::Pong, 0, 10, &[1]);
    let mut stream = vec![MAGIC[0]];
    stream.extend_from_slice(&frame);
    let mut parser = FrameParser::new();

    let (frames, errors) = feed_collect(&mut parser, &stream);

    assert!(errors.is_empty());
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].msg_type, MessageType::Pong);
    assert_eq!(frames[0].seq, 10);
    assert_eq!(frames[0].payload, vec![1]);
}

#[test]
fn parser_handles_multiple_h_prefixes() {
    let frame = make_frame(MessageType::Ack, 0, 11, &[2, 3]);
    let mut stream = vec![MAGIC[0], MAGIC[0], MAGIC[0]];
    stream.extend_from_slice(&frame);
    let mut parser = FrameParser::new();

    let (frames, errors) = feed_collect(&mut parser, &stream);

    assert!(errors.is_empty());
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].msg_type, MessageType::Ack);
    assert_eq!(frames[0].seq, 11);
    assert_eq!(frames[0].payload, vec![2, 3]);
}

#[test]
fn parser_rejects_corrupt_payload_crc() {
    let mut frame = make_frame(MessageType::Pong, 0, 12, &[1, 2, 3, 4]);
    frame[HEADER_LEN + 2] ^= 0x01;
    let mut parser = FrameParser::new();

    let (frames, errors) = feed_collect(&mut parser, &frame);

    assert!(frames.is_empty());
    assert_eq!(errors, vec![ProtocolError::CrcMismatch]);

    let next = make_frame(MessageType::Ping, 0, 13, &[]);
    let (frames, errors) = feed_collect(&mut parser, &next);
    assert!(errors.is_empty());
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].seq, 13);
}

#[test]
fn parser_rejects_corrupt_header_crc() {
    let mut frame = make_frame(MessageType::Ping, 0, 14, &[]);
    frame[8] ^= 0x01;
    let mut parser = FrameParser::new();

    let (frames, errors) = feed_collect(&mut parser, &frame);

    assert!(frames.is_empty());
    assert_eq!(errors, vec![ProtocolError::CrcMismatch]);

    let next = make_frame(MessageType::Ping, 0, 15, &[]);
    let (frames, errors) = feed_collect(&mut parser, &next);
    assert!(errors.is_empty());
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].seq, 15);
}

#[test]
fn parser_recovers_after_crc_failure() {
    let first = make_frame(MessageType::Ping, 0, 16, &[]);
    let mut corrupt = make_frame(MessageType::Pong, 0, 17, &[1, 2, 3, 4]);
    let third = make_frame(MessageType::Ack, 0, 18, &[9, 9, 9, 9]);
    corrupt[HEADER_LEN + 1] ^= 0x55;
    let stream =
        [first.as_slice(), corrupt.as_slice(), third.as_slice()].concat();
    let mut parser = FrameParser::new();

    let (frames, errors) = feed_collect(&mut parser, &stream);

    assert_eq!(errors, vec![ProtocolError::CrcMismatch]);
    assert_eq!(frames.len(), 2);
    assert_eq!(frames[0].msg_type, MessageType::Ping);
    assert_eq!(frames[0].seq, 16);
    assert_eq!(frames[1].msg_type, MessageType::Ack);
    assert_eq!(frames[1].seq, 18);
}

#[test]
fn parser_enters_discard_on_oversized_payload() {
    let payload = vec![0xa5; MAX_PAYLOAD_SIZE + 1];
    let frame = make_oversized_frame(&payload, 19);
    let mut parser = FrameParser::new();

    for &byte in &frame[..HEADER_LEN] {
        assert_eq!(parser.feed_byte(byte), FeedResult::NeedMore);
    }

    for &byte in &frame[HEADER_LEN..frame.len() - 1] {
        assert_eq!(parser.feed_byte(byte), FeedResult::NeedMore);
    }

    assert_eq!(
        parser.feed_byte(*frame.last().unwrap()),
        FeedResult::Error(ProtocolError::OversizedPayload)
    );
    assert!(parser.take_frame().is_none());
}

#[test]
fn parser_recovers_after_oversized_frame() {
    let inner_valid = make_frame(MessageType::Pong, 0, 20, &[1, 2, 3]);
    let final_valid = make_frame(MessageType::Ping, 0, 21, &[]);
    let mut oversized_payload = vec![0x55; MAX_PAYLOAD_SIZE + 1];
    oversized_payload[..inner_valid.len()].copy_from_slice(&inner_valid);
    let oversized = make_oversized_frame(&oversized_payload, 22);
    let stream = [oversized.as_slice(), final_valid.as_slice()].concat();
    let mut parser = FrameParser::new();

    let (frames, errors) = feed_collect(&mut parser, &stream);

    assert_eq!(errors, vec![ProtocolError::OversizedPayload]);
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].msg_type, MessageType::Ping);
    assert_eq!(frames[0].seq, 21);
}

#[test]
fn parser_rejects_reserved_flags() {
    let mut frame = make_frame(MessageType::Ping, 0, 23, &[]);
    frame[4] = 0b1000_0000;
    let mut parser = FrameParser::new();

    let (frames, errors) = feed_collect(&mut parser, &frame);

    assert!(frames.is_empty());
    assert_eq!(errors, vec![ProtocolError::ReservedNonZero]);
}

#[test]
fn parser_rejects_reserved_header_byte() {
    let mut frame = make_frame(MessageType::Ping, 0, 24, &[]);
    frame[5] = 1;
    let mut parser = FrameParser::new();

    let (frames, errors) = feed_collect(&mut parser, &frame);

    assert!(frames.is_empty());
    assert_eq!(errors, vec![ProtocolError::ReservedNonZero]);

    let next = make_frame(MessageType::Ping, 0, 25, &[]);
    let (frames, errors) = feed_collect(&mut parser, &next);
    assert!(errors.is_empty());
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].seq, 25);
}

#[test]
fn parser_rejects_reserved_state_field_bits() {
    let mut payload = [0u8; STATE_DELTA_FIXED_LEN];
    let invalid_mask = VALID_STATE_FIELDS | (1 << 9);
    payload[0..2].copy_from_slice(&invalid_mask.to_le_bytes());
    let mut state = full_state();
    let before = state;

    let err = decode_state_delta(&payload, &mut state).unwrap_err();

    assert_eq!(err, ProtocolError::InvalidFieldMask);
    assert_eq!(state, before);
}

#[test]
fn decoder_rejects_truncated_fixed_section() {
    let mut state = UiState::new();
    let err = decode_state_delta(&[0; STATE_DELTA_FIXED_LEN - 1], &mut state)
        .unwrap_err();

    assert_eq!(err, ProtocolError::InvalidLength);
}

#[test]
fn decoder_rejects_truncated_variable_string() {
    let mut payload = [0u8; STATE_DELTA_FIXED_LEN + 4];
    payload[0..2].copy_from_slice(&FIELD_TITLE.to_le_bytes());
    payload[23] = 20;
    payload[STATE_DELTA_FIXED_LEN..].copy_from_slice(b"tiny");
    let mut state = UiState::new();

    let err = decode_state_delta(&payload, &mut state).unwrap_err();

    assert_eq!(err, ProtocolError::InvalidLength);
}

#[test]
fn decoder_rejects_payload_claiming_more_than_available() {
    let mask = FIELD_TITLE | FIELD_ARTIST | FIELD_ALBUM | FIELD_APP_NAME;
    let mut payload = [0u8; STATE_DELTA_FIXED_LEN + 8];
    payload[0..2].copy_from_slice(&mask.to_le_bytes());
    payload[23] = 4;
    payload[24] = 4;
    payload[25] = 4;
    payload[26] = 4;
    payload[STATE_DELTA_FIXED_LEN..].copy_from_slice(b"12345678");
    let mut state = UiState::new();

    let err = decode_state_delta(&payload, &mut state).unwrap_err();

    assert_eq!(err, ProtocolError::InvalidLength);
}

#[test]
fn decoder_accepts_valid_utf8() {
    let mut state = UiState::new();
    state.title.set("naïve café".as_bytes()).unwrap();
    state.artist.set("坂本龍一".as_bytes()).unwrap();
    state.album.set("🌈".as_bytes()).unwrap();
    state.app_name.set("Música".as_bytes()).unwrap();
    let mask = FIELD_TITLE | FIELD_ARTIST | FIELD_ALBUM | FIELD_APP_NAME;
    let mut payload = [0u8; 256];
    let len = encode_state_delta(&state, mask, &mut payload).unwrap();
    let mut decoded = UiState::new();

    let decoded_mask =
        decode_state_delta(&payload[..len], &mut decoded).unwrap();

    assert_eq!(decoded_mask, mask);
    assert_eq!(decoded.title.as_str(), "naïve café");
    assert_eq!(decoded.artist.as_str(), "坂本龍一");
    assert_eq!(decoded.album.as_str(), "🌈");
    assert_eq!(decoded.app_name.as_str(), "Música");
}

#[test]
fn decoder_rejects_invalid_utf8() {
    let mask = FIELD_TRACK_ID | FIELD_TITLE;
    let mut payload = [0u8; STATE_DELTA_FIXED_LEN + 2];
    payload[0..2].copy_from_slice(&mask.to_le_bytes());
    payload[2..6].copy_from_slice(&5u32.to_le_bytes());
    payload[23] = 2;
    payload[STATE_DELTA_FIXED_LEN..].copy_from_slice(&[0xc3, 0x28]);
    let mut state = full_state();
    let before = state;

    let err = decode_state_delta(&payload, &mut state).unwrap_err();

    assert_eq!(err, ProtocolError::InvalidUtf8);
    assert_eq!(state, before);
}

#[test]
fn hstr_truncation_preserves_utf8_boundary() {
    let mut value = HStr::<5>::new();

    value.truncate("abcdé".as_bytes());

    assert!(core::str::from_utf8(value.as_bytes()).is_ok());
    assert_eq!(value.as_bytes(), b"abcd");
}

#[test]
fn state_delta_roundtrip() {
    let state = full_state();
    let mask = all_state_fields();
    let mut payload = [0u8; MAX_PAYLOAD_SIZE];
    let len = encode_state_delta(&state, mask, &mut payload).unwrap();
    let mut decoded = UiState::new();

    let decoded_mask =
        decode_state_delta(&payload[..len], &mut decoded).unwrap();

    assert_eq!(decoded_mask, mask);
    assert_eq!(decoded, state);
}

#[test]
fn frame_roundtrip() {
    let state = full_state();
    let mask = all_state_fields();
    let mut payload = [0u8; MAX_PAYLOAD_SIZE];
    let payload_len = encode_state_delta(&state, mask, &mut payload).unwrap();
    let frame =
        make_frame(MessageType::StateDelta, 0, 26, &payload[..payload_len]);
    let mut parser = FrameParser::new();

    let (frames, errors) = feed_collect(&mut parser, &frame);

    assert!(errors.is_empty());
    assert_eq!(frames.len(), 1);
    assert_eq!(frames[0].msg_type, MessageType::StateDelta);
    assert_eq!(frames[0].seq, 26);

    let mut decoded = UiState::new();
    let decoded_mask =
        decode_state_delta(&frames[0].payload, &mut decoded).unwrap();
    assert_eq!(decoded_mask, mask);
    assert_eq!(decoded, state);
}

#[test]
fn seq_distance_handles_wraparound() {
    assert_eq!(seq_distance(0, 65_535), 1);
}

#[test]
fn seq_distance_detects_old_duplicate() {
    assert_eq!(seq_distance(41, 42), -1);
    assert!(seq_distance(100, 200) < 0);
}

#[test]
fn parser_never_panics_on_random_bytes() {
    let mut rng = Lcg::new(0x1234_5678);
    let mut parser = FrameParser::new();

    for _ in 0..10_000 {
        let result = parser.feed_byte(rng.next_u8());
        if result == FeedResult::FrameReady {
            let _ = parser.take_frame();
        }
    }
}

#[test]
fn parser_handles_long_randomized_stream() {
    let mut rng = Lcg::new(0xa11c_e55d);
    let mut stream = Vec::new();
    let mut expected_valid_seqs = Vec::new();

    for seq in 100u16..140 {
        for _ in 0..(rng.next_u8() % 5) {
            stream.push(rng.next_u8());
        }

        let valid = make_frame(MessageType::Ping, 0, seq, &[rng.next_u8()]);
        stream.extend_from_slice(&valid);
        expected_valid_seqs.push(seq);

        let mut corrupt = make_frame(
            MessageType::Pong,
            0,
            seq.wrapping_add(1000),
            &[1, 2, 3],
        );
        corrupt[HEADER_LEN] ^= 0x80;
        stream.extend_from_slice(&corrupt);

        let truncated = make_frame(
            MessageType::Ack,
            0,
            seq.wrapping_add(2000),
            &[4, 5, 6, 7],
        );
        stream.extend_from_slice(&truncated[..truncated.len() - 2]);
        stream.extend_from_slice(&[0, 1, 2, 3, 4, 5, 6, 7]);

        let mut oversized_payload = vec![0x33; MAX_PAYLOAD_SIZE + 1];
        if seq % 7 == 0 {
            let embedded =
                make_frame(MessageType::Pong, 0, seq.wrapping_add(3000), &[]);
            oversized_payload[..embedded.len()].copy_from_slice(&embedded);
        }
        let oversized =
            make_oversized_frame(&oversized_payload, seq.wrapping_add(4000));
        stream.extend_from_slice(&oversized);
    }

    let sentinel = make_frame(MessageType::Resync, 0, 999, &[]);
    stream.extend_from_slice(&sentinel);
    expected_valid_seqs.push(999);

    let mut parser = FrameParser::new();
    let (frames, _errors) = feed_collect(&mut parser, &stream);
    let actual_valid_seqs: Vec<u16> =
        frames.iter().map(|frame| frame.seq).collect();

    assert_eq!(actual_valid_seqs, expected_valid_seqs);
}

struct Lcg {
    state: u32,
}

impl Lcg {
    const fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    fn next_u8(&mut self) -> u8 {
        self.state = self
            .state
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        (self.state >> 24) as u8
    }
}
