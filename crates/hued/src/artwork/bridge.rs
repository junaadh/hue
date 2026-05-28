use std::os::raw::{c_char, c_int};

#[repr(C)]
pub struct HueArtworkResult {
    pub ok: c_int,
    pub width: u16,
    pub height: u16,
    pub len: usize,
    pub data: *mut u8,
}

unsafe extern "C" {
    pub fn hue_artwork_process(
        path: *const c_char,
        size: u16,
    ) -> HueArtworkResult;

    pub fn hue_artwork_free(ptr: *mut u8, len: usize);
}
