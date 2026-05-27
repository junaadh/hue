use hue_core::hash::Fnv32;
use hue_protocol::{
    ArtworkRx, ArtworkRxStatus,
    payload::{
        ARTWORK_ABORT_TRACK_CHANGED, ARTWORK_FORMAT_RGB565,
        ArtworkAbortPayload, ArtworkBeginPayload, ArtworkChunkPayload,
        ArtworkEndPayload,
    },
    wire::ProtocolError,
};

fn begin_payload() -> ArtworkBeginPayload {
    ArtworkBeginPayload {
        artwork_id: 0x1234_5678,
        width: 4,
        height: 2,
        total_bytes: 16,
        format: ARTWORK_FORMAT_RGB565,
    }
}

fn checksum(chunks: &[&[u8]]) -> u32 {
    let mut checksum = Fnv32::new();
    for chunk in chunks {
        checksum.feed_bytes(chunk);
    }
    checksum.finish()
}

#[test]
fn artwork_rx_accepts_ordered_stream_and_resets_after_end() {
    let first = [0, 1, 2, 3, 4, 5, 6, 7];
    let second = [8, 9, 10, 11, 12, 13, 14, 15];
    let mut rx = ArtworkRx::new();

    assert_eq!(
        rx.begin(begin_payload()).unwrap(),
        ArtworkRxStatus::Began {
            artwork_id: 0x1234_5678,
            total_bytes: 16,
        }
    );
    assert!(rx.is_active());
    assert_eq!(rx.artwork_id(), Some(0x1234_5678));
    assert_eq!(rx.expected_chunk_index(), 0);

    assert_eq!(
        rx.chunk(ArtworkChunkPayload {
            artwork_id: 0x1234_5678,
            chunk_index: 0,
            data: &first,
        })
        .unwrap(),
        ArtworkRxStatus::ChunkAccepted {
            artwork_id: 0x1234_5678,
            chunk_index: 0,
            received_bytes: 8,
            total_bytes: 16,
        }
    );
    assert_eq!(rx.expected_chunk_index(), 1);
    assert_eq!(rx.total_chunks(), 1);
    assert_eq!(rx.received_bytes(), 8);

    rx.chunk(ArtworkChunkPayload {
        artwork_id: 0x1234_5678,
        chunk_index: 1,
        data: &second,
    })
    .unwrap();

    let expected_checksum = checksum(&[&first, &second]);
    assert_eq!(rx.checksum(), expected_checksum);
    assert_eq!(rx.total_chunks(), 2);

    assert_eq!(
        rx.end(ArtworkEndPayload {
            artwork_id: 0x1234_5678,
            checksum: expected_checksum,
        })
        .unwrap(),
        ArtworkRxStatus::Complete {
            artwork_id: 0x1234_5678,
            total_chunks: 2,
            total_bytes: 16,
            checksum: expected_checksum,
        }
    );

    assert!(!rx.is_active());
    assert_eq!(rx.artwork_id(), None);
    assert_eq!(rx.received_bytes(), 0);
}

#[test]
fn artwork_rx_rejects_chunk_before_begin() {
    let mut rx = ArtworkRx::new();
    let data = [0, 1];

    let err = rx
        .chunk(ArtworkChunkPayload {
            artwork_id: 1,
            chunk_index: 0,
            data: &data,
        })
        .unwrap_err();

    assert_eq!(err, ProtocolError::InvalidLength);
}

#[test]
fn artwork_rx_rejects_begin_while_active() {
    let mut rx = ArtworkRx::new();

    rx.begin(begin_payload()).unwrap();

    assert_eq!(
        rx.begin(begin_payload()).unwrap_err(),
        ProtocolError::InvalidLength
    );
}

#[test]
fn artwork_rx_detects_wrong_artwork_id() {
    let mut rx = ArtworkRx::new();
    let data = [0, 1];

    rx.begin(begin_payload()).unwrap();

    let err = rx
        .chunk(ArtworkChunkPayload {
            artwork_id: 0xffff_ffff,
            chunk_index: 0,
            data: &data,
        })
        .unwrap_err();

    assert_eq!(err, ProtocolError::ArtworkIdMismatch);
    assert!(rx.is_active());
    assert_eq!(rx.expected_chunk_index(), 0);
}

#[test]
fn artwork_rx_detects_wrong_chunk_index() {
    let mut rx = ArtworkRx::new();
    let data = [0, 1];

    rx.begin(begin_payload()).unwrap();

    let err = rx
        .chunk(ArtworkChunkPayload {
            artwork_id: 0x1234_5678,
            chunk_index: 1,
            data: &data,
        })
        .unwrap_err();

    assert_eq!(err, ProtocolError::ChunkIndexMismatch);
    assert_eq!(rx.expected_chunk_index(), 0);
    assert_eq!(rx.received_bytes(), 0);
}

#[test]
fn artwork_rx_rejects_odd_chunk_data() {
    let mut rx = ArtworkRx::new();
    let data = [0, 1, 2];

    rx.begin(begin_payload()).unwrap();

    let err = rx
        .chunk(ArtworkChunkPayload {
            artwork_id: 0x1234_5678,
            chunk_index: 0,
            data: &data,
        })
        .unwrap_err();

    assert_eq!(err, ProtocolError::InvalidAlignment);
    assert_eq!(rx.received_bytes(), 0);
}

#[test]
fn artwork_rx_rejects_more_bytes_than_declared() {
    let mut rx = ArtworkRx::new();
    let data = [0u8; 18];

    rx.begin(begin_payload()).unwrap();

    let err = rx
        .chunk(ArtworkChunkPayload {
            artwork_id: 0x1234_5678,
            chunk_index: 0,
            data: &data,
        })
        .unwrap_err();

    assert_eq!(err, ProtocolError::InvalidLength);
    assert_eq!(rx.received_bytes(), 0);
}

#[test]
fn artwork_rx_rejects_end_before_all_bytes_arrive() {
    let mut rx = ArtworkRx::new();
    let data = [0, 1];

    rx.begin(begin_payload()).unwrap();
    rx.chunk(ArtworkChunkPayload {
        artwork_id: 0x1234_5678,
        chunk_index: 0,
        data: &data,
    })
    .unwrap();

    let err = rx
        .end(ArtworkEndPayload {
            artwork_id: 0x1234_5678,
            checksum: checksum(&[&data]),
        })
        .unwrap_err();

    assert_eq!(err, ProtocolError::InvalidLength);
    assert!(rx.is_active());
}

#[test]
fn artwork_rx_detects_checksum_mismatch() {
    let mut rx = ArtworkRx::new();
    let data = [0u8; 16];

    rx.begin(begin_payload()).unwrap();
    rx.chunk(ArtworkChunkPayload {
        artwork_id: 0x1234_5678,
        chunk_index: 0,
        data: &data,
    })
    .unwrap();

    let err = rx
        .end(ArtworkEndPayload {
            artwork_id: 0x1234_5678,
            checksum: 0,
        })
        .unwrap_err();

    assert_eq!(err, ProtocolError::ChecksumMismatch);
    assert!(rx.is_active());
}

#[test]
fn artwork_rx_abort_resets_active_stream() {
    let mut rx = ArtworkRx::new();

    rx.begin(begin_payload()).unwrap();

    assert_eq!(
        rx.abort(ArtworkAbortPayload {
            artwork_id: 0x1234_5678,
            reason: ARTWORK_ABORT_TRACK_CHANGED,
        })
        .unwrap(),
        ArtworkRxStatus::Aborted {
            artwork_id: 0x1234_5678,
            reason: ARTWORK_ABORT_TRACK_CHANGED,
        }
    );

    assert!(!rx.is_active());
    assert_eq!(rx.expected_chunk_index(), 0);
}

#[test]
fn artwork_rx_abort_rejects_unknown_reason() {
    let mut rx = ArtworkRx::new();

    rx.begin(begin_payload()).unwrap();

    let err = rx
        .abort(ArtworkAbortPayload {
            artwork_id: 0x1234_5678,
            reason: 0xff,
        })
        .unwrap_err();

    assert_eq!(err, ProtocolError::InvalidLength);
    assert!(rx.is_active());
}
