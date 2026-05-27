use hue_core::hstr::HString;

use crate::{cstr, sys};

pub(crate) static EVENT_HANDLER: std::sync::OnceLock<
    Box<dyn Fn(MediaEvent) + Send + Sync>,
> = std::sync::OnceLock::new();

pub(crate) extern "C" fn event_trampoline(raw: *const sys::HueEventRaw) {
    if raw.is_null() {
        return;
    }

    let event = unsafe { MediaEvent::from_raw(&*raw) };

    if let Some(handler) = EVENT_HANDLER.get() {
        handler(event);
    }
}

#[derive(Debug, Clone)]
pub enum MediaEvent {
    Playback { playing: bool },
    TrackChanged(PlaybackSnapshot),
    Metadata(PlaybackSnapshot),
}

#[derive(Debug, Clone)]
pub struct PlaybackSnapshot {
    pub song: SongSnapshot,
    pub progress: ProgressSnapshot,
    pub playing: bool,
    pub artwork_path: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SongSnapshot {
    pub title: HString,
    pub artist: HString,
    pub album: HString,
    pub app_name: HString,
}

#[derive(Debug, Clone, Copy)]
pub struct ProgressSnapshot {
    pub position: f64,
    pub duration: f64,
}

impl PlaybackSnapshot {
    fn from_raw(raw: &sys::HuePlaybackInfoRaw) -> Self {
        Self {
            song: SongSnapshot {
                title: HString::from_str(cstr(raw.song_info.title)),
                artist: HString::from_str(cstr(raw.song_info.artist)),
                album: HString::from_str(cstr(raw.song_info.album)),
                app_name: HString::from_str(cstr(raw.song_info.app_name)),
            },
            progress: ProgressSnapshot {
                position: raw.progress_info.position,
                duration: raw.progress_info.duration,
            },
            playing: raw.playing != 0,
            artwork_path: {
                let path = cstr(raw.artwork_info.path);
                if path.is_empty() {
                    None
                } else {
                    Some(path.to_owned())
                }
            },
        }
    }
}

impl MediaEvent {
    unsafe fn from_raw(raw: &sys::HueEventRaw) -> Self {
        match raw.kind {
            sys::HueEventKindRaw::Playback => {
                let ev = unsafe { raw.data.playback };
                Self::Playback {
                    playing: ev.playing != 0,
                }
            }

            sys::HueEventKindRaw::TrackChanged => {
                let ev = unsafe { &raw.data.changed };
                Self::TrackChanged(PlaybackSnapshot::from_raw(
                    &ev.playback_info,
                ))
            }

            sys::HueEventKindRaw::Metadata => {
                let ev = unsafe { &raw.data.metadata };
                Self::Metadata(PlaybackSnapshot::from_raw(&ev.playback_info))
            }
        }
    }
}
