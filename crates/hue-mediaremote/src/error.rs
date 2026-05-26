use std::fmt;

#[derive(Debug)]
pub enum HueError {
    NullOutput,
    AppleScriptFailed,
    ParseFailed,
    ArtworkFailed,
    MediaRemoteLoadFailed,
    MediaRemoteCommandMissing,
    Unknown(i32),
}

impl std::error::Error for HueError {}

impl HueError {
    pub(crate) const fn from_code(code: i32) -> Self {
        match code {
            -1 => Self::MediaRemoteLoadFailed,
            -2 => Self::MediaRemoteCommandMissing,
            -10 => Self::NullOutput,
            -20 => Self::AppleScriptFailed,
            -21 => Self::ParseFailed,
            -22 => Self::ArtworkFailed,
            other => Self::Unknown(other),
        }
    }
}

impl fmt::Display for HueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NullOutput => {
                write!(f, "received null output pointer")
            }
            Self::AppleScriptFailed => {
                write!(f, "AppleScript execution failed")
            }
            Self::ParseFailed => {
                write!(f, "failed to parse playback information")
            }
            Self::ArtworkFailed => {
                write!(f, "failed to retrieve artwork")
            }
            Self::MediaRemoteLoadFailed => {
                write!(f, "failed to load MediaRemote framework")
            }
            Self::MediaRemoteCommandMissing => {
                write!(f, "MediaRemote command symbol missing")
            }
            Self::Unknown(code) => {
                write!(f, "unknown hue-mediaremote error ({code})")
            }
        }
    }
}
