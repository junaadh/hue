use hue_core::state::UiState;
use hue_protocol::delta::diff_state;
use hue_protocol::payload::{
    FIELD_ARTIST, FIELD_ARTWORK_ID, FIELD_DURATION, FIELD_PLAYING,
    FIELD_POSITION, FIELD_TITLE, FIELD_TRACK_ID, decode_state_delta,
    encode_state_delta,
};

#[test]
fn decode_basic_state_delta() {
    let mut payload = [0u8; 32];

    let mask = FIELD_TRACK_ID | FIELD_POSITION | FIELD_PLAYING;

    payload[0..2].copy_from_slice(&mask.to_le_bytes());

    payload[2..6].copy_from_slice(&123u32.to_le_bytes());

    payload[6..14].copy_from_slice(&5000u64.to_le_bytes());

    payload[22] = 1;

    let mut state = UiState::new();

    let decoded_mask = decode_state_delta(&payload, &mut state).unwrap();

    assert_eq!(decoded_mask, mask);

    assert_eq!(state.track_id, 123);
    assert_eq!(state.position_ms, 5000);
    assert!(state.playing);
}

#[test]
fn state_delta_roundtrip() {
    let mut state = UiState::new();

    state.track_id = 99;
    state.artwork_id = 1001;
    state.title.set(b"Nights").unwrap();
    state.artist.set(b"Frank Ocean").unwrap();
    state.position_ms = 12_000;
    state.duration_ms = 300_000;
    state.playing = true;

    let mask = FIELD_TRACK_ID
        | FIELD_TITLE
        | FIELD_ARTIST
        | FIELD_ARTWORK_ID
        | FIELD_POSITION
        | FIELD_DURATION
        | FIELD_PLAYING;

    let mut payload = [0u8; 256];

    let len = encode_state_delta(&state, mask, &mut payload).unwrap();

    let mut decoded = UiState::new();

    let decoded_mask =
        decode_state_delta(&payload[..len], &mut decoded).unwrap();

    assert_eq!(decoded_mask, mask);
    assert_eq!(decoded.track_id, 99);
    assert_eq!(decoded.artwork_id, 1001);
    assert_eq!(decoded.title.as_str(), "Nights");
    assert_eq!(decoded.artist.as_str(), "Frank Ocean");
    assert_eq!(decoded.position_ms, 12_000);
    assert_eq!(decoded.duration_ms, 300_000);
    assert!(decoded.playing);
}

#[test]
fn diff_state_includes_artwork_id_for_new_track() {
    let old = UiState::new();
    let mut new = UiState::new();
    new.track_id = 99;
    new.artwork_id = 1001;

    let mask = diff_state(&old, &new);

    assert_ne!(mask & FIELD_TRACK_ID, 0);
    assert_ne!(mask & FIELD_ARTWORK_ID, 0);
}

#[test]
fn diff_state_includes_artwork_id_for_same_track_artwork_change() {
    let mut old = UiState::new();
    old.track_id = 99;
    old.artwork_id = 1001;
    let mut new = old;
    new.artwork_id = 1002;

    assert_eq!(diff_state(&old, &new), FIELD_ARTWORK_ID);
}
