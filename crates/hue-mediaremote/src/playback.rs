use crate::{
    ProgressInfo, SongInfo,
    artwork::{ArtworkInfo, ArtworkKind},
    progress::ProgressKind,
    song::SongKind,
    sys,
};

#[allow(dead_code)]
pub(crate) enum PlaybackKind<'a> {
    Owned(sys::HuePlaybackInfoRaw),
    Borrowed(&'a sys::HuePlaybackInfoRaw),
}

pub struct PlaybackInfo<'a> {
    pub(crate) kind: PlaybackKind<'a>,
}

impl PlaybackInfo<'_> {
    #[inline(always)]
    const fn raw(&self) -> &sys::HuePlaybackInfoRaw {
        match &self.kind {
            PlaybackKind::Owned(o) => o,
            PlaybackKind::Borrowed(b) => b,
        }
    }

    pub fn song(&self) -> SongInfo<'_> {
        SongInfo {
            kind: SongKind::Borrowed(&self.raw().song_info),
        }
    }

    pub fn progress(&self) -> ProgressInfo<'_> {
        ProgressInfo {
            kind: ProgressKind::Borrowed(&self.raw().progress_info),
        }
    }

    pub fn artwork(&self) -> ArtworkInfo<'_> {
        ArtworkInfo {
            kind: ArtworkKind::Borrowed(&self.raw().artwork_info),
        }
    }

    pub fn is_playing(&self) -> bool {
        self.raw().playing != 0
    }
}

impl Drop for PlaybackInfo<'_> {
    fn drop(&mut self) {
        if let PlaybackKind::Owned(owned) = &mut self.kind {
            unsafe { sys::hue_free_playback_info(owned) };
        }
    }
}
