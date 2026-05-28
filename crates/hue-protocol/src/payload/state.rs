use crate::{Result, wire::ProtocolError};
use hue_core::{hstr::HStrError, state::UiState};

pub const STATE_DELTA_FIXED_LEN: usize = 32;

pub const FIELD_TRACK_ID: u16 = 1 << 0;
pub const FIELD_TITLE: u16 = 1 << 1;
pub const FIELD_ARTIST: u16 = 1 << 2;
pub const FIELD_ALBUM: u16 = 1 << 3;
pub const FIELD_APP_NAME: u16 = 1 << 4;
pub const FIELD_POSITION: u16 = 1 << 5;
pub const FIELD_DURATION: u16 = 1 << 6;
pub const FIELD_PLAYING: u16 = 1 << 7;
pub const FIELD_ARTWORK_ID: u16 = 1 << 8;

pub const VALID_STATE_FIELDS: u16 = FIELD_TRACK_ID
    | FIELD_TITLE
    | FIELD_ARTIST
    | FIELD_ALBUM
    | FIELD_APP_NAME
    | FIELD_POSITION
    | FIELD_DURATION
    | FIELD_PLAYING
    | FIELD_ARTWORK_ID;

pub const RESERVED_STATE_FIELDS: u16 = !VALID_STATE_FIELDS;

///
/// +0   u16 field_mask
/// +2   u32 track_id
///
/// +6   u64 position_ms
///
/// +14  u64 duration_ms
///
/// +22  u8  playing
/// +23  u8  title_len
/// +24  u8  artist_len
/// +25  u8  album_len
/// +26  u8  app_name_len
///
/// +27  u8  reserved
/// +28  u32 artwork_id
///
/// +32  variable string data starts
///
pub fn decode_state_delta(payload: &[u8], out: &mut UiState) -> Result<u16> {
    if payload.len() < STATE_DELTA_FIXED_LEN {
        return Err(ProtocolError::InvalidLength);
    }

    let mut next = *out;

    let field_mask = u16::from_le_bytes([payload[0], payload[1]]);

    if field_mask & RESERVED_STATE_FIELDS != 0 {
        return Err(ProtocolError::InvalidFieldMask);
    }

    if payload[27] != 0 {
        return Err(ProtocolError::ReservedNonZero);
    }

    if field_mask & FIELD_TRACK_ID != 0 {
        next.track_id = u32::from_le_bytes([
            payload[2], payload[3], payload[4], payload[5],
        ]);
    }

    if field_mask & FIELD_POSITION != 0 {
        next.position_ms =
            u64::from_le_bytes(payload[6..14].try_into().unwrap());
    }

    if field_mask & FIELD_DURATION != 0 {
        next.duration_ms =
            u64::from_le_bytes(payload[14..22].try_into().unwrap());
    }

    if field_mask & FIELD_PLAYING != 0 {
        next.playing = match payload[22] {
            0 => false,
            1 => true,
            _ => return Err(ProtocolError::InvalidLength),
        };
    }

    let title_len = payload[23] as usize;
    let artist_len = payload[24] as usize;
    let album_len = payload[25] as usize;
    let app_name_len = payload[26] as usize;

    if field_mask & FIELD_ARTWORK_ID != 0 {
        next.artwork_id =
            u32::from_le_bytes(payload[28..32].try_into().unwrap());
    }

    let mut cursor = STATE_DELTA_FIXED_LEN;

    if field_mask & FIELD_TITLE != 0 {
        let bytes = take_bytes(payload, &mut cursor, title_len)?;
        next.title.set(bytes).map_err(map_hstr_error)?;
    }

    if field_mask & FIELD_ARTIST != 0 {
        let bytes = take_bytes(payload, &mut cursor, artist_len)?;
        next.artist.set(bytes).map_err(map_hstr_error)?;
    }

    if field_mask & FIELD_ALBUM != 0 {
        let bytes = take_bytes(payload, &mut cursor, album_len)?;
        next.album.set(bytes).map_err(map_hstr_error)?;
    }

    if field_mask & FIELD_APP_NAME != 0 {
        let bytes = take_bytes(payload, &mut cursor, app_name_len)?;
        next.app_name.set(bytes).map_err(map_hstr_error)?;
    }

    *out = next;

    Ok(field_mask)
}

fn take_bytes<'a>(
    payload: &'a [u8],

    cursor: &mut usize,

    len: usize,
) -> Result<&'a [u8]> {
    let end = cursor
        .checked_add(len)
        .ok_or(ProtocolError::InvalidLength)?;

    if end > payload.len() {
        return Err(ProtocolError::InvalidLength);
    }

    let bytes = &payload[*cursor..end];

    *cursor = end;

    Ok(bytes)
}

fn map_hstr_error(err: HStrError) -> ProtocolError {
    match err {
        HStrError::UTF8Error => ProtocolError::InvalidUtf8,
        HStrError::TooLong => ProtocolError::OversizedPayload,
    }
}

pub fn encode_state_delta(
    state: &UiState,
    field_mask: u16,
    out: &mut [u8],
) -> Result<usize> {
    if field_mask & RESERVED_STATE_FIELDS != 0 {
        return Err(ProtocolError::InvalidFieldMask);
    }

    if out.len() < STATE_DELTA_FIXED_LEN {
        return Err(ProtocolError::InvalidLength);
    }

    out[..STATE_DELTA_FIXED_LEN].fill(0);

    out[0..2].copy_from_slice(&field_mask.to_le_bytes());

    if field_mask & FIELD_TRACK_ID != 0 {
        out[2..6].copy_from_slice(&state.track_id.to_le_bytes());
    }

    if field_mask & FIELD_POSITION != 0 {
        out[6..14].copy_from_slice(&state.position_ms.to_le_bytes());
    }

    if field_mask & FIELD_DURATION != 0 {
        out[14..22].copy_from_slice(&state.duration_ms.to_le_bytes());
    }

    if field_mask & FIELD_PLAYING != 0 {
        out[22] = u8::from(state.playing);
    }

    if field_mask & FIELD_ARTWORK_ID != 0 {
        out[28..32].copy_from_slice(&state.artwork_id.to_le_bytes());
    }

    let title = state.title.as_bytes();
    let artist = state.artist.as_bytes();
    let album = state.album.as_bytes();
    let app_name = state.app_name.as_bytes();

    let mut cursor = STATE_DELTA_FIXED_LEN;

    if field_mask & FIELD_TITLE != 0 {
        out[23] = title.len() as u8;
        write_bytes(out, &mut cursor, title)?;
    }

    if field_mask & FIELD_ARTIST != 0 {
        out[24] = artist.len() as u8;
        write_bytes(out, &mut cursor, artist)?;
    }

    if field_mask & FIELD_ALBUM != 0 {
        out[25] = album.len() as u8;
        write_bytes(out, &mut cursor, album)?;
    }

    if field_mask & FIELD_APP_NAME != 0 {
        out[26] = app_name.len() as u8;
        write_bytes(out, &mut cursor, app_name)?;
    }

    Ok(cursor)
}

fn write_bytes(out: &mut [u8], cursor: &mut usize, bytes: &[u8]) -> Result<()> {
    let end = cursor
        .checked_add(bytes.len())
        .ok_or(ProtocolError::InvalidLength)?;

    if end > out.len() {
        return Err(ProtocolError::InvalidLength);
    }

    out[*cursor..end].copy_from_slice(bytes);
    *cursor = end;

    Ok(())
}
