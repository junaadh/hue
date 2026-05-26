use std::ffi::{CStr, c_char};

pub use error::HueError;
pub use event::{MediaEvent, PlaybackSnapshot, ProgressSnapshot, SongSnapshot};
pub use mr::MediaRemote;
pub use playback::PlaybackInfo;
pub use progress::ProgressInfo;
pub use song::SongInfo;

mod artwork;
mod error;
mod event;
mod mr;
mod playback;
mod progress;
mod song;
mod sys;

pub type Result<T> = std::result::Result<T, HueError>;

fn check(code: i32) -> Result<()> {
    if code == 0 {
        Ok(())
    } else {
        Err(HueError::from_code(code))
    }
}

fn cstr<'a>(ptr: *const c_char) -> &'a str {
    if ptr.is_null() {
        return "";
    }

    unsafe { CStr::from_ptr(ptr).to_str().unwrap_or("") }
}
