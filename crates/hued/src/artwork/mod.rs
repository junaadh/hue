pub mod bridge;

use std::{ffi::CString, slice};

use hue_core::hash::Fnv32;

pub const ARTWORK_SIZE: u16 = 160;

#[derive(Debug, Clone)]
pub struct ProcessedArtwork {
    pub artwork_id: u32,
    pub width: u16,
    pub height: u16,
    pub rgb565: Vec<u8>,
}

#[derive(Debug)]
pub enum ArtworkError {
    InvalidPath,
    ProcessingFailed,
}

pub fn process_artwork(path: &str) -> Result<ProcessedArtwork, ArtworkError> {
    let path = CString::new(path).map_err(|_| ArtworkError::InvalidPath)?;

    let result =
        unsafe { bridge::hue_artwork_process(path.as_ptr(), ARTWORK_SIZE) };

    if result.ok == 0 || result.data.is_null() || result.len == 0 {
        return Err(ArtworkError::ProcessingFailed);
    }

    let rgb565 = unsafe {
        let bytes = slice::from_raw_parts(result.data, result.len);
        let out = bytes.to_vec();
        bridge::hue_artwork_free(result.data, result.len);
        out
    };

    let mut hash = Fnv32::new();
    hash.feed_bytes(&rgb565);

    Ok(ProcessedArtwork {
        artwork_id: hash.finish(),
        width: result.width,
        height: result.height,
        rgb565,
    })
}
