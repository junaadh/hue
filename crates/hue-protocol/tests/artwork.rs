use hue_protocol::{
    payload::{
        ARTWORK_ABORT_ENCODER_ERROR, ARTWORK_ABORT_SENDER_REQUEST,
        ARTWORK_ABORT_TRACK_CHANGED, ARTWORK_FORMAT_RGB565,
        ArtworkAbortPayload, ArtworkBeginPayload, ArtworkChunkPayload,
        ArtworkEndPayload,
    },
    wire::ProtocolError,
};

#[test]
fn artwork_begin_roundtrip() {
    let begin = ArtworkBeginPayload {
        artwork_id: 0x1234_5678,
        width: 4,
        height: 3,
        total_bytes: 24,
        format: ARTWORK_FORMAT_RGB565,
    };
    let mut buf = [0xff; ArtworkBeginPayload::LEN];

    let len = begin.encode(&mut buf).unwrap();
    let decoded = ArtworkBeginPayload::decode(&buf).unwrap();

    assert_eq!(len, ArtworkBeginPayload::LEN);
    assert_eq!(decoded, begin);
    assert_eq!(&buf[0..4], &begin.artwork_id.to_le_bytes());
    assert_eq!(&buf[4..6], &begin.width.to_le_bytes());
    assert_eq!(&buf[6..8], &begin.height.to_le_bytes());
    assert_eq!(&buf[8..12], &begin.total_bytes.to_le_bytes());
    assert_eq!(buf[12], ARTWORK_FORMAT_RGB565);
    assert!(buf[13..24].iter().all(|&byte| byte == 0));
}

#[test]
fn artwork_begin_rejects_zero_width_or_height() {
    let mut buf = [0; ArtworkBeginPayload::LEN];
    let mut begin = ArtworkBeginPayload {
        artwork_id: 1,
        width: 0,
        height: 3,
        total_bytes: 0,
        format: ARTWORK_FORMAT_RGB565,
    };

    assert_eq!(
        begin.encode(&mut buf).unwrap_err(),
        ProtocolError::InvalidLength
    );

    begin.width = 4;
    begin.height = 0;
    assert_eq!(
        begin.encode(&mut buf).unwrap_err(),
        ProtocolError::InvalidLength
    );
}

#[test]
fn artwork_begin_rejects_total_bytes_mismatch() {
    let mut buf = [0; ArtworkBeginPayload::LEN];
    let begin = ArtworkBeginPayload {
        artwork_id: 1,
        width: 4,
        height: 3,
        total_bytes: 22,
        format: ARTWORK_FORMAT_RGB565,
    };

    assert_eq!(
        begin.encode(&mut buf).unwrap_err(),
        ProtocolError::InvalidLength
    );
}

#[test]
fn artwork_begin_rejects_invalid_format() {
    let mut buf = [0; ArtworkBeginPayload::LEN];
    let begin = ArtworkBeginPayload {
        artwork_id: 1,
        width: 4,
        height: 3,
        total_bytes: 24,
        format: 1,
    };

    assert_eq!(
        begin.encode(&mut buf).unwrap_err(),
        ProtocolError::InvalidLength
    );
}

#[test]
fn artwork_begin_rejects_reserved_bytes() {
    let begin = ArtworkBeginPayload {
        artwork_id: 1,
        width: 4,
        height: 3,
        total_bytes: 24,
        format: ARTWORK_FORMAT_RGB565,
    };
    let mut buf = [0; ArtworkBeginPayload::LEN];
    begin.encode(&mut buf).unwrap();
    buf[13] = 1;

    assert_eq!(
        ArtworkBeginPayload::decode(&buf).unwrap_err(),
        ProtocolError::ReservedNonZero
    );
}

#[test]
fn artwork_chunk_roundtrip() {
    let data = [0xaa, 0xbb, 0xcc, 0xdd];
    let chunk = ArtworkChunkPayload {
        artwork_id: 0x0102_0304,
        chunk_index: 7,
        data: &data,
    };
    let mut buf = [0; ArtworkChunkPayload::HEADER_LEN + 4];

    let len = chunk.encode(&mut buf).unwrap();
    let decoded = ArtworkChunkPayload::decode(&buf).unwrap();

    assert_eq!(len, ArtworkChunkPayload::HEADER_LEN + data.len());
    assert_eq!(decoded, chunk);
    assert_eq!(&buf[0..4], &chunk.artwork_id.to_le_bytes());
    assert_eq!(&buf[4..6], &chunk.chunk_index.to_le_bytes());
    assert_eq!(&buf[6..8], &(data.len() as u16).to_le_bytes());
    assert_eq!(&buf[8..], data);
}

#[test]
fn artwork_chunk_rejects_odd_data_length() {
    let data = [1, 2, 3];
    let chunk = ArtworkChunkPayload {
        artwork_id: 1,
        chunk_index: 0,
        data: &data,
    };
    let mut buf = [0; ArtworkChunkPayload::HEADER_LEN + 3];

    assert_eq!(
        chunk.encode(&mut buf).unwrap_err(),
        ProtocolError::InvalidAlignment
    );
}

#[test]
fn artwork_chunk_rejects_payload_length_mismatch() {
    let mut buf = [0; ArtworkChunkPayload::HEADER_LEN + 2];
    buf[6..8].copy_from_slice(&4u16.to_le_bytes());

    assert_eq!(
        ArtworkChunkPayload::decode(&buf).unwrap_err(),
        ProtocolError::InvalidLength
    );
}

#[test]
fn artwork_end_roundtrip() {
    let end = ArtworkEndPayload {
        artwork_id: 0x0a0b_0c0d,
        checksum: 0xdead_beef,
    };
    let mut buf = [0; ArtworkEndPayload::LEN];

    let len = end.encode(&mut buf).unwrap();
    let decoded = ArtworkEndPayload::decode(&buf).unwrap();

    assert_eq!(len, ArtworkEndPayload::LEN);
    assert_eq!(decoded, end);
}

#[test]
fn artwork_abort_roundtrip_for_all_known_reasons() {
    for reason in [
        ARTWORK_ABORT_TRACK_CHANGED,
        ARTWORK_ABORT_ENCODER_ERROR,
        ARTWORK_ABORT_SENDER_REQUEST,
    ] {
        let abort = ArtworkAbortPayload {
            artwork_id: 0x1111_2222,
            reason,
        };
        let mut buf = [0xff; ArtworkAbortPayload::LEN];

        let len = abort.encode(&mut buf).unwrap();
        let decoded = ArtworkAbortPayload::decode(&buf).unwrap();

        assert_eq!(len, ArtworkAbortPayload::LEN);
        assert_eq!(decoded, abort);
        assert!(buf[5..8].iter().all(|&byte| byte == 0));
    }
}

#[test]
fn artwork_abort_rejects_unknown_reason() {
    let abort = ArtworkAbortPayload {
        artwork_id: 1,
        reason: 0xff,
    };
    let mut buf = [0; ArtworkAbortPayload::LEN];

    assert_eq!(
        abort.encode(&mut buf).unwrap_err(),
        ProtocolError::InvalidLength
    );
}

#[test]
fn artwork_abort_rejects_reserved_bytes() {
    let abort = ArtworkAbortPayload {
        artwork_id: 1,
        reason: ARTWORK_ABORT_TRACK_CHANGED,
    };
    let mut buf = [0; ArtworkAbortPayload::LEN];
    abort.encode(&mut buf).unwrap();
    buf[7] = 1;

    assert_eq!(
        ArtworkAbortPayload::decode(&buf).unwrap_err(),
        ProtocolError::ReservedNonZero
    );
}
