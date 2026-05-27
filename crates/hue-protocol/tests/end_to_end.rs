use hue_core::{hash::Fnv32, state::UiState};
use hue_protocol::{
    ArtworkRx, ArtworkRxStatus, SeqStatus, SeqTracker,
    builder::FrameBuilder,
    parser::{FeedResult, FrameParser},
    payload::{
        ARTWORK_FORMAT_RGB565, ArtworkBeginPayload, ArtworkChunkPayload,
        ArtworkEndPayload, CAPS_ARTWORK, CAPS_COMMANDS, CAPS_DELTA,
        HandshakePayload, decode_state_delta,
    },
    wire::{HEADER_LEN, MAX_PAYLOAD_SIZE, MessageType},
};

#[derive(Debug, PartialEq, Eq)]
struct CapturedFrame {
    msg_type: MessageType,
    flags: u8,
    seq: u16,
    payload: Vec<u8>,
}

fn push_frame(stream: &mut Vec<u8>, frame_buf: &mut [u8], len: usize) {
    stream.extend_from_slice(&frame_buf[..len]);
}

fn parse_stream(stream: &[u8]) -> Vec<CapturedFrame> {
    let mut parser = FrameParser::new();
    let mut frames = Vec::new();

    for &byte in stream {
        match parser.feed_byte(byte) {
            FeedResult::FrameReady => {
                let frame = parser.take_frame().unwrap();
                frames.push(CapturedFrame {
                    msg_type: frame.header.msg_type,
                    flags: frame.header.flags,
                    seq: frame.header.seq,
                    payload: frame.payload.to_vec(),
                });
            }
            FeedResult::NeedMore => {}
            FeedResult::Error(err) => {
                panic!("unexpected parser error: {err:?}")
            }
        }
    }

    frames
}

fn full_track_state() -> UiState {
    let mut state = UiState::new();
    state.track_id = 0xfeed_beef;
    state.title.set(b"Protocol Song").unwrap();
    state.artist.set(b"Hue").unwrap();
    state.album.set(b"Wire v0").unwrap();
    state.app_name.set(b"hued").unwrap();
    state.position_ms = 0;
    state.duration_ms = 180_000;
    state.playing = true;
    state
}

fn checksum(chunks: &[&[u8]]) -> u32 {
    let mut checksum = Fnv32::new();
    for chunk in chunks {
        checksum.feed_bytes(chunk);
    }
    checksum.finish()
}

#[test]
fn media_state_plus_artwork_stream_end_to_end() {
    let mut stream = Vec::new();
    let mut frame_buf = [0u8; HEADER_LEN + MAX_PAYLOAD_SIZE];
    let mut seq = 0u16;

    let handshake = HandshakePayload {
        proto_version: 0,
        min_proto_version: 0,
        caps: CAPS_ARTWORK | CAPS_COMMANDS | CAPS_DELTA,
        epoch: 0x1234_5678,
    };
    let len = FrameBuilder::handshake(&mut frame_buf, seq, handshake).unwrap();
    push_frame(&mut stream, &mut frame_buf, len);
    seq += 1;

    let empty_state = UiState::new();
    let track_state = full_track_state();
    let len = FrameBuilder::state_delta(
        &mut frame_buf,
        seq,
        &empty_state,
        &track_state,
    )
    .unwrap();
    push_frame(&mut stream, &mut frame_buf, len);
    seq += 1;

    let artwork_begin = ArtworkBeginPayload {
        artwork_id: track_state.track_id,
        width: 4,
        height: 2,
        total_bytes: 16,
        format: ARTWORK_FORMAT_RGB565,
    };
    let len = FrameBuilder::artwork_begin(&mut frame_buf, seq, artwork_begin)
        .unwrap();
    push_frame(&mut stream, &mut frame_buf, len);
    seq += 1;

    let chunk0 = [0, 1, 2, 3, 4, 5, 6, 7];
    let chunk1 = [8, 9, 10, 11, 12, 13, 14, 15];
    let len = FrameBuilder::artwork_chunk(
        &mut frame_buf,
        seq,
        ArtworkChunkPayload {
            artwork_id: track_state.track_id,
            chunk_index: 0,
            data: &chunk0,
        },
    )
    .unwrap();
    push_frame(&mut stream, &mut frame_buf, len);
    seq += 1;

    let len = FrameBuilder::artwork_chunk(
        &mut frame_buf,
        seq,
        ArtworkChunkPayload {
            artwork_id: track_state.track_id,
            chunk_index: 1,
            data: &chunk1,
        },
    )
    .unwrap();
    push_frame(&mut stream, &mut frame_buf, len);
    seq += 1;

    let artwork_checksum = checksum(&[&chunk0, &chunk1]);
    let len = FrameBuilder::artwork_end(
        &mut frame_buf,
        seq,
        ArtworkEndPayload {
            artwork_id: track_state.track_id,
            checksum: artwork_checksum,
        },
    )
    .unwrap();
    push_frame(&mut stream, &mut frame_buf, len);
    seq += 1;

    let mut positioned_state = track_state;
    positioned_state.position_ms = 10_000;
    let len = FrameBuilder::state_delta(
        &mut frame_buf,
        seq,
        &track_state,
        &positioned_state,
    )
    .unwrap();
    push_frame(&mut stream, &mut frame_buf, len);

    let frames = parse_stream(&stream);
    assert_eq!(frames.len(), 7);

    let mut seq_tracker = SeqTracker::new(0);
    let mut state = UiState::new();
    let mut artwork_rx = ArtworkRx::new();

    for frame in &frames {
        assert_eq!(seq_tracker.observe(frame.seq), SeqStatus::Expected);
        assert_eq!(frame.flags, 0);
    }

    assert_eq!(frames[0].msg_type, MessageType::Handshake);
    assert_eq!(
        HandshakePayload::decode(&frames[0].payload).unwrap(),
        handshake
    );

    assert_eq!(frames[1].msg_type, MessageType::StateDelta);
    decode_state_delta(&frames[1].payload, &mut state).unwrap();
    assert_eq!(state, track_state);

    assert_eq!(frames[2].msg_type, MessageType::ArtworkBegin);
    assert_eq!(
        artwork_rx
            .begin(ArtworkBeginPayload::decode(&frames[2].payload).unwrap())
            .unwrap(),
        ArtworkRxStatus::Began {
            artwork_id: track_state.track_id,
            total_bytes: 16,
        }
    );

    assert_eq!(frames[3].msg_type, MessageType::ArtworkChunk);
    artwork_rx
        .chunk(ArtworkChunkPayload::decode(&frames[3].payload).unwrap())
        .unwrap();

    assert_eq!(frames[4].msg_type, MessageType::ArtworkChunk);
    artwork_rx
        .chunk(ArtworkChunkPayload::decode(&frames[4].payload).unwrap())
        .unwrap();

    assert_eq!(frames[5].msg_type, MessageType::ArtworkEnd);
    assert_eq!(
        artwork_rx
            .end(ArtworkEndPayload::decode(&frames[5].payload).unwrap())
            .unwrap(),
        ArtworkRxStatus::Complete {
            artwork_id: track_state.track_id,
            total_chunks: 2,
            total_bytes: 16,
            checksum: artwork_checksum,
        }
    );
    assert!(!artwork_rx.is_active());

    assert_eq!(frames[6].msg_type, MessageType::StateDelta);
    decode_state_delta(&frames[6].payload, &mut state).unwrap();
    assert_eq!(state, positioned_state);
}
