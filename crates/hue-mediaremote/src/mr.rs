use crate::{
    PlaybackInfo, ProgressInfo, Result,
    artwork::{ArtworkInfo, ArtworkKind},
    check,
    event::{EVENT_HANDLER, MediaEvent, event_trampoline},
    playback::PlaybackKind,
    progress::ProgressKind,
    sys,
};

pub struct MediaRemote;

impl MediaRemote {
    pub fn playback_info(&self) -> Result<PlaybackInfo<'_>> {
        let mut raw = sys::HuePlaybackInfoRaw::zeroed();

        let code = unsafe { sys::hue_get_playback_info(&mut raw) };
        check(code)?;

        Ok(PlaybackInfo {
            kind: PlaybackKind::Owned(raw),
        })
    }

    pub fn progress(&self) -> Result<ProgressInfo<'_>> {
        let mut raw = sys::HueProgressInfoRaw::default();

        let code = unsafe { sys::hue_get_progress(&mut raw) };
        check(code)?;

        Ok(ProgressInfo {
            kind: ProgressKind::Owned(raw),
        })
    }

    pub fn artwork(&self) -> Result<ArtworkInfo<'_>> {
        let mut raw = sys::HueArtworkInfoRaw {
            path: std::ptr::null(),
        };

        let code = unsafe { sys::hue_get_artwork(&mut raw) };
        check(code)?;

        Ok(ArtworkInfo {
            kind: ArtworkKind::Owned(raw),
        })
    }

    pub fn play_pause(&self) -> Result<()> {
        check(unsafe { sys::hue_play_pause() })
    }

    pub fn next_track(&self) -> Result<()> {
        check(unsafe { sys::hue_next_track() })
    }

    pub fn previous_track(&self) -> Result<()> {
        check(unsafe { sys::hue_previous_track() })
    }

    pub fn register_events<F>(&self, handler: F) -> Result<()>
    where
        F: Fn(MediaEvent) + Send + Sync + 'static,
    {
        let _ = EVENT_HANDLER.set(Box::new(handler));

        check(unsafe { sys::hue_register_notifications(event_trampoline) })
    }

    pub fn run_loop(&self) -> ! {
        unsafe { sys::hue_run_loop() }
    }
}
