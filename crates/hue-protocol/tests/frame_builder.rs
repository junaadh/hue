use hue_core::state::UiState;
use hue_protocol::{
    builder::FrameBuilder,
    parser::{FeedResult, FrameParser},
    payload::{
        ARTWORK_ABORT_SENDER_REQUEST, ARTWORK_FORMAT_RGB565, AckPayload,
        ArtworkAbortPayload, ArtworkBeginPayload, ArtworkChunkPayload,
        ArtworkEndPayload, CAPS_COMMANDS, CAPS_DELTA, FIELD_PLAYING,
        FIELD_POSITION, HandshakePayload, NackPayload, STATE_DELTA_FIXED_LEN,
        decode_state_delta,
    },
    wire::{
        FLAG_IS_RESP, HEADER_LEN, MAX_PAYLOAD_SIZE, MessageType, ProtocolError,
    },
};

fn parse_one(frame: &[u8]) -> (MessageType, u8, u16, Vec<u8>) {
    let mut parser = FrameParser::new();

    for &byte in &frame[..frame.len() - 1] {
        assert_eq!(parser.feed_byte(byte), FeedResult::NeedMore);
    }

    assert_eq!(
        parser.feed_byte(*frame.last().unwrap()),
        FeedResult::FrameReady
    );

    let parsed = parser.take_frame().unwrap();
    (
        parsed.header.msg_type,
        parsed.header.flags,
        parsed.header.seq,
        parsed.payload.to_vec(),
    )
}

fn populated_state() -> UiState {
    let mut state = UiState::new();
    state.track_id = 42;
    state.title.set(b"Track").unwrap();
    state.artist.set(b"Artist").unwrap();
    state.album.set(b"Album").unwrap();
    state.app_name.set(b"App").unwrap();
    state.position_ms = 1_000;
    state.duration_ms = 120_000;
    state.playing = false;
    state
}

#[test]
fn frame_builder_ping_builds_empty_ping_frame() {
    let mut frame_buf = [0u8; HEADER_LEN];

    let len = FrameBuilder::ping(&mut frame_buf, 1).unwrap();
    let (msg_type, flags, seq, payload) = parse_one(&frame_buf[..len]);

    assert_eq!(len, HEADER_LEN);
    assert_eq!(msg_type, MessageType::Ping);
    assert_eq!(flags, 0);
    assert_eq!(seq, 1);
    assert!(payload.is_empty());
}

#[test]
fn frame_builder_pong_builds_empty_pong_frame() {
    let mut frame_buf = [0u8; HEADER_LEN];

    let len = FrameBuilder::pong(&mut frame_buf, 2).unwrap();
    let (msg_type, flags, seq, payload) = parse_one(&frame_buf[..len]);

    assert_eq!(len, HEADER_LEN);
    assert_eq!(msg_type, MessageType::Pong);
    assert_eq!(flags, 0);
    assert_eq!(seq, 2);
    assert!(payload.is_empty());
}

#[test]
fn frame_builder_resync_builds_empty_resync_frame() {
    let mut frame_buf = [0u8; HEADER_LEN];

    let len = FrameBuilder::resync(&mut frame_buf, 3).unwrap();
    let (msg_type, flags, seq, payload) = parse_one(&frame_buf[..len]);

    assert_eq!(len, HEADER_LEN);
    assert_eq!(msg_type, MessageType::Resync);
    assert_eq!(flags, 0);
    assert_eq!(seq, 3);
    assert!(payload.is_empty());
}

#[test]
fn frame_builder_empty_control_frames_reject_short_buffer() {
    let mut frame_buf = [0u8; HEADER_LEN - 1];

    assert_eq!(
        FrameBuilder::ping(&mut frame_buf, 4).unwrap_err(),
        ProtocolError::InvalidLength
    );
}

#[test]
fn frame_builder_state_delta_roundtrip_decodes_to_new_state() {
    let old_state = UiState::new();
    let mut new_state = populated_state();
    new_state.playing = true;

    let mut frame_buf = [0u8; HEADER_LEN + MAX_PAYLOAD_SIZE];

    let len =
        FrameBuilder::state_delta(&mut frame_buf, 77, &old_state, &new_state)
            .unwrap();
    let (msg_type, flags, seq, payload) = parse_one(&frame_buf[..len]);

    assert_eq!(msg_type, MessageType::StateDelta);
    assert_eq!(flags, 0);
    assert_eq!(seq, 77);

    let mut decoded = old_state;
    decode_state_delta(&payload, &mut decoded).unwrap();

    assert_eq!(decoded, new_state);
}

#[test]
fn frame_builder_state_delta_uses_diff_mask() {
    let old_state = populated_state();
    let mut new_state = old_state;
    new_state.position_ms = 2_000;
    new_state.playing = true;

    let mut frame_buf = [0u8; HEADER_LEN + MAX_PAYLOAD_SIZE];

    let len =
        FrameBuilder::state_delta(&mut frame_buf, 78, &old_state, &new_state)
            .unwrap();
    let (msg_type, flags, seq, payload) = parse_one(&frame_buf[..len]);

    assert_eq!(msg_type, MessageType::StateDelta);
    assert_eq!(flags, 0);
    assert_eq!(seq, 78);
    assert_eq!(payload.len(), STATE_DELTA_FIXED_LEN);

    let mut decoded = old_state;
    let mask = decode_state_delta(&payload, &mut decoded).unwrap();

    assert_eq!(mask, FIELD_POSITION | FIELD_PLAYING);
    assert_eq!(decoded, new_state);
}

#[test]
fn frame_builder_rejects_buffer_shorter_than_header() {
    let old_state = UiState::new();
    let new_state = populated_state();
    let mut frame_buf = [0u8; HEADER_LEN - 1];

    let err =
        FrameBuilder::state_delta(&mut frame_buf, 79, &old_state, &new_state)
            .unwrap_err();

    assert_eq!(err, ProtocolError::InvalidLength);
}

#[test]
fn frame_builder_ack_wraps_ack_payload_as_response() {
    let mut frame_buf = [0u8; HEADER_LEN + AckPayload::LEN];

    let len = FrameBuilder::ack(&mut frame_buf, 81, 77).unwrap();
    let (msg_type, flags, seq, payload) = parse_one(&frame_buf[..len]);

    assert_eq!(len, HEADER_LEN + AckPayload::LEN);
    assert_eq!(msg_type, MessageType::Ack);
    assert_eq!(flags, FLAG_IS_RESP);
    assert_eq!(seq, 81);
    assert_eq!(AckPayload::decode(&payload).unwrap().acked_seq, 77);
}

#[test]
fn frame_builder_handshake_wraps_payload_without_response_flag() {
    let payload = HandshakePayload {
        proto_version: 1,
        min_proto_version: 0,
        caps: CAPS_COMMANDS | CAPS_DELTA,
        epoch: 0xfeed_beef,
    };
    let mut frame_buf = [0u8; HEADER_LEN + HandshakePayload::LEN];

    let len = FrameBuilder::handshake(&mut frame_buf, 85, payload).unwrap();
    let (msg_type, flags, seq, frame_payload) = parse_one(&frame_buf[..len]);

    assert_eq!(len, HEADER_LEN + HandshakePayload::LEN);
    assert_eq!(msg_type, MessageType::Handshake);
    assert_eq!(flags, 0);
    assert_eq!(seq, 85);
    assert_eq!(HandshakePayload::decode(&frame_payload).unwrap(), payload);
}

#[test]
fn frame_builder_handshake_rejects_reserved_caps() {
    let payload = HandshakePayload {
        proto_version: 1,
        min_proto_version: 0,
        caps: 0x8000,
        epoch: 0,
    };
    let mut frame_buf = [0u8; HEADER_LEN + HandshakePayload::LEN];

    let err = FrameBuilder::handshake(&mut frame_buf, 86, payload).unwrap_err();

    assert_eq!(err, ProtocolError::ReservedNonZero);
}

#[test]
fn frame_builder_artwork_begin_wraps_payload() {
    let begin = ArtworkBeginPayload {
        artwork_id: 0x1234_5678,
        width: 4,
        height: 3,
        total_bytes: 24,
        format: ARTWORK_FORMAT_RGB565,
    };
    let mut frame_buf = [0u8; HEADER_LEN + ArtworkBeginPayload::LEN];

    let len = FrameBuilder::artwork_begin(&mut frame_buf, 90, begin).unwrap();
    let (msg_type, flags, seq, payload) = parse_one(&frame_buf[..len]);

    assert_eq!(len, HEADER_LEN + ArtworkBeginPayload::LEN);
    assert_eq!(msg_type, MessageType::ArtworkBegin);
    assert_eq!(flags, 0);
    assert_eq!(seq, 90);
    assert_eq!(ArtworkBeginPayload::decode(&payload).unwrap(), begin);
}

#[test]
fn frame_builder_artwork_chunk_wraps_payload_without_extra_copy() {
    let data = [0xaa, 0xbb, 0xcc, 0xdd];
    let chunk = ArtworkChunkPayload {
        artwork_id: 0x1234_5678,
        chunk_index: 2,
        data: &data,
    };
    let mut frame_buf = [0u8; HEADER_LEN + ArtworkChunkPayload::HEADER_LEN + 4];

    let len = FrameBuilder::artwork_chunk(&mut frame_buf, 91, chunk).unwrap();
    let (msg_type, flags, seq, payload) = parse_one(&frame_buf[..len]);

    assert_eq!(
        len,
        HEADER_LEN + ArtworkChunkPayload::HEADER_LEN + data.len()
    );
    assert_eq!(msg_type, MessageType::ArtworkChunk);
    assert_eq!(flags, 0);
    assert_eq!(seq, 91);
    assert_eq!(ArtworkChunkPayload::decode(&payload).unwrap(), chunk);
    assert_eq!(
        &frame_buf[HEADER_LEN + ArtworkChunkPayload::HEADER_LEN..len],
        data
    );
}

#[test]
fn frame_builder_artwork_end_wraps_payload() {
    let end = ArtworkEndPayload {
        artwork_id: 0x1234_5678,
        checksum: 0xdead_beef,
    };
    let mut frame_buf = [0u8; HEADER_LEN + ArtworkEndPayload::LEN];

    let len = FrameBuilder::artwork_end(&mut frame_buf, 92, end).unwrap();
    let (msg_type, flags, seq, payload) = parse_one(&frame_buf[..len]);

    assert_eq!(len, HEADER_LEN + ArtworkEndPayload::LEN);
    assert_eq!(msg_type, MessageType::ArtworkEnd);
    assert_eq!(flags, 0);
    assert_eq!(seq, 92);
    assert_eq!(ArtworkEndPayload::decode(&payload).unwrap(), end);
}

#[test]
fn frame_builder_artwork_abort_wraps_payload() {
    let abort = ArtworkAbortPayload {
        artwork_id: 0x1234_5678,
        reason: ARTWORK_ABORT_SENDER_REQUEST,
    };
    let mut frame_buf = [0u8; HEADER_LEN + ArtworkAbortPayload::LEN];

    let len = FrameBuilder::artwork_abort(&mut frame_buf, 93, abort).unwrap();
    let (msg_type, flags, seq, payload) = parse_one(&frame_buf[..len]);

    assert_eq!(len, HEADER_LEN + ArtworkAbortPayload::LEN);
    assert_eq!(msg_type, MessageType::ArtworkAbort);
    assert_eq!(flags, 0);
    assert_eq!(seq, 93);
    assert_eq!(ArtworkAbortPayload::decode(&payload).unwrap(), abort);
}

#[test]
fn frame_builder_artwork_chunk_rejects_short_payload_buffer() {
    let data = [0xaa, 0xbb, 0xcc, 0xdd];
    let chunk = ArtworkChunkPayload {
        artwork_id: 1,
        chunk_index: 0,
        data: &data,
    };
    let mut frame_buf = [0u8; HEADER_LEN + ArtworkChunkPayload::HEADER_LEN + 3];

    let err =
        FrameBuilder::artwork_chunk(&mut frame_buf, 94, chunk).unwrap_err();

    assert_eq!(err, ProtocolError::InvalidLength);
}

#[test]
fn frame_builder_nack_wraps_nack_payload_as_response() {
    let mut frame_buf = [0u8; HEADER_LEN + NackPayload::LEN];

    let len = FrameBuilder::nack(
        &mut frame_buf,
        82,
        78,
        ProtocolError::InvalidLength,
        MessageType::StateDelta,
    )
    .unwrap();
    let (msg_type, flags, seq, payload) = parse_one(&frame_buf[..len]);
    let nack = NackPayload::decode(&payload).unwrap();

    assert_eq!(len, HEADER_LEN + NackPayload::LEN);
    assert_eq!(msg_type, MessageType::Nack);
    assert_eq!(flags, FLAG_IS_RESP);
    assert_eq!(seq, 82);
    assert_eq!(nack.nacked_seq, 78);
    assert_eq!(nack.error_code, ProtocolError::InvalidLength as u8);
    assert_eq!(nack.context_type, MessageType::StateDelta as u8);
}

#[test]
fn frame_builder_ack_rejects_short_payload_buffer() {
    let mut frame_buf = [0u8; HEADER_LEN + AckPayload::LEN - 1];

    let err = FrameBuilder::ack(&mut frame_buf, 83, 79).unwrap_err();

    assert_eq!(err, ProtocolError::InvalidLength);
}

#[test]
fn frame_builder_nack_rejects_short_payload_buffer() {
    let mut frame_buf = [0u8; HEADER_LEN + NackPayload::LEN - 1];

    let err = FrameBuilder::nack(
        &mut frame_buf,
        84,
        80,
        ProtocolError::CrcMismatch,
        MessageType::Pong,
    )
    .unwrap_err();

    assert_eq!(err, ProtocolError::InvalidLength);
}

#[test]
fn frame_builder_rejects_buffer_without_fixed_payload_space() {
    let old_state = UiState::new();
    let new_state = UiState::new();
    let mut frame_buf = [0u8; HEADER_LEN + STATE_DELTA_FIXED_LEN - 1];

    let err =
        FrameBuilder::state_delta(&mut frame_buf, 80, &old_state, &new_state)
            .unwrap_err();

    assert_eq!(err, ProtocolError::InvalidLength);
}
