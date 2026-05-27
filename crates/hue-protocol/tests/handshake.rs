use hue_protocol::{
    payload::{CAPS_ARTWORK, CAPS_COMMANDS, CAPS_DELTA, HandshakePayload},
    wire::ProtocolError,
};

#[test]
fn handshake_roundtrip() {
    let hs = HandshakePayload {
        proto_version: 1,
        min_proto_version: 0,
        caps: CAPS_ARTWORK | CAPS_COMMANDS | CAPS_DELTA,
        epoch: 0x12345678,
    };

    let mut buf = [0u8; 8];

    hs.encode(&mut buf).unwrap();

    let decoded = HandshakePayload::decode(&buf).unwrap();

    assert_eq!(decoded, hs);
}

#[test]
fn reserved_caps_are_rejected() {
    let hs = HandshakePayload {
        proto_version: 1,
        min_proto_version: 0,
        caps: 0x8000,
        epoch: 0,
    };

    let mut buf = [0u8; 8];

    let err = hs.encode(&mut buf).unwrap_err();

    assert_eq!(err, ProtocolError::ReservedNonZero);
}
