use crate::{hash::track_id_from_parts, hstr::HString};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiState {
    pub track_id: u32,

    pub title: HString,
    pub album: HString,
    pub artist: HString,
    pub app_name: HString,

    pub position_ms: u64,
    pub duration_ms: u64,

    pub playing: bool,
}

impl UiState {
    pub const fn new() -> Self {
        Self {
            track_id: 0,
            title: HString::new(),
            album: HString::new(),
            artist: HString::new(),
            app_name: HString::new(),
            position_ms: 0,
            duration_ms: 0,
            playing: false,
        }
    }

    pub fn recompute_track_id(&mut self) {
        self.track_id = track_id_from_parts(
            self.title.as_str(),
            self.artist.as_str(),
            self.album.as_str(),
            self.duration_ms,
        );
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self::new()
    }
}
