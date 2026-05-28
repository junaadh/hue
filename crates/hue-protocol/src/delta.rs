use crate::payload::{
    FIELD_ALBUM, FIELD_APP_NAME, FIELD_ARTIST, FIELD_ARTWORK_ID,
    FIELD_DURATION, FIELD_PLAYING, FIELD_POSITION, FIELD_TITLE, FIELD_TRACK_ID,
};
use hue_core::state::UiState;

pub fn diff_state(old: &UiState, new: &UiState) -> u16 {
    let mut mask = 0u16;

    if old.track_id != new.track_id {
        // New track: force full metadata refresh.
        mask |= FIELD_TRACK_ID
            | FIELD_TITLE
            | FIELD_ARTIST
            | FIELD_ALBUM
            | FIELD_APP_NAME
            | FIELD_DURATION
            | FIELD_ARTWORK_ID;

        // Position usually resets or jumps on new track too.
        if old.position_ms != new.position_ms {
            mask |= FIELD_POSITION;
        }

        if old.playing != new.playing {
            mask |= FIELD_PLAYING;
        }

        return mask;
    }

    // Same track: only send cheap/high-frequency fields unless metadata truly changed.
    if old.position_ms != new.position_ms {
        mask |= FIELD_POSITION;
    }

    if old.artwork_id != new.artwork_id {
        mask |= FIELD_ARTWORK_ID;
    }

    if old.playing != new.playing {
        mask |= FIELD_PLAYING;
    }

    if old.duration_ms != new.duration_ms {
        mask |= FIELD_DURATION;
    }

    // Rare correction/update from media source.
    if old.title != new.title {
        mask |= FIELD_TITLE;
    }

    if old.artist != new.artist {
        mask |= FIELD_ARTIST;
    }

    if old.album != new.album {
        mask |= FIELD_ALBUM;
    }

    if old.app_name != new.app_name {
        mask |= FIELD_APP_NAME;
    }

    mask
}
