use hue_protocol::payload::{AckPayload, NackPayload};

#[test]
fn ack_roundtrip() {
    let ack = AckPayload { acked_seq: 42 };

    let mut buf = [0u8; 4];

    ack.encode(&mut buf).unwrap();

    let decoded = AckPayload::decode(&buf).unwrap();

    assert_eq!(decoded, ack);
}

#[test]
fn nack_roundtrip() {
    let nack = NackPayload {
        nacked_seq: 7,
        error_code: 9,
        context_type: 5,
    };

    let mut buf = [0u8; 4];

    nack.encode(&mut buf).unwrap();

    let decoded = NackPayload::decode(&buf).unwrap();

    assert_eq!(decoded, nack);
}
