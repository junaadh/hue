use std::ffi::c_char;

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct HueSongInfoRaw {
    pub(crate) title: *const c_char,
    pub(crate) artist: *const c_char,
    pub(crate) album: *const c_char,
    pub(crate) app_name: *const c_char,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct HueProgressInfoRaw {
    pub(crate) position: f64,
    pub(crate) duration: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct HueArtworkInfoRaw {
    pub(crate) path: *const c_char,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct HuePlaybackInfoRaw {
    pub(crate) song_info: HueSongInfoRaw,
    pub(crate) progress_info: HueProgressInfoRaw,
    pub(crate) playing: i32,
    pub(crate) artwork_info: HueArtworkInfoRaw,
}

impl HuePlaybackInfoRaw {
    pub(crate) const fn zeroed() -> Self {
        Self {
            song_info: HueSongInfoRaw {
                title: std::ptr::null(),
                artist: std::ptr::null(),
                album: std::ptr::null(),
                app_name: std::ptr::null(),
            },
            progress_info: HueProgressInfoRaw {
                position: 0.0,
                duration: 0.0,
            },
            playing: 0,
            artwork_info: HueArtworkInfoRaw {
                path: std::ptr::null(),
            },
        }
    }
}

#[allow(dead_code)]
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HueEventKindRaw {
    Playback = 0,
    TrackChanged = 1,
    Metadata = 2,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) union HueEventDataRaw {
    pub(crate) playback: HuePlaybackEventRaw,
    pub(crate) changed: HueChangedEventRaw,
    pub(crate) metadata: HueMetadataEventRaw,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct HuePlaybackEventRaw {
    pub(crate) playing: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct HueChangedEventRaw {
    pub(crate) playback_info: HuePlaybackInfoRaw,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct HueMetadataEventRaw {
    pub(crate) playback_info: HuePlaybackInfoRaw,
}

#[repr(C)]
pub(crate) struct HueEventRaw {
    pub(crate) kind: HueEventKindRaw,
    pub(crate) data: HueEventDataRaw,
}

pub(crate) type HueEventCallback = extern "C" fn(*const HueEventRaw);

unsafe extern "C" {
    pub(crate) fn hue_get_playback_info(out: *mut HuePlaybackInfoRaw) -> i32;
    pub(crate) fn hue_free_playback_info(info: *mut HuePlaybackInfoRaw);

    pub(crate) fn hue_get_progress(out: *mut HueProgressInfoRaw) -> i32;

    pub(crate) fn hue_get_artwork(out: *mut HueArtworkInfoRaw) -> i32;
    pub(crate) fn hue_free_artwork(out: *mut HueArtworkInfoRaw);

    pub(crate) fn hue_register_notifications(cb: HueEventCallback) -> i32;
    pub(crate) fn hue_run_loop() -> !;

    pub(crate) fn hue_play_pause() -> i32;
    pub(crate) fn hue_next_track() -> i32;
    pub(crate) fn hue_previous_track() -> i32;
}
