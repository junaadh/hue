use crate::{cstr, sys};
use std::path::Path;

pub(crate) enum ArtworkKind<'a> {
    Owned(sys::HueArtworkInfoRaw),
    Borrowed(&'a sys::HueArtworkInfoRaw),
}

pub struct ArtworkInfo<'a> {
    pub(crate) kind: ArtworkKind<'a>,
}

impl ArtworkInfo<'_> {
    #[inline(always)]
    const fn raw(&self) -> &sys::HueArtworkInfoRaw {
        match &self.kind {
            ArtworkKind::Owned(o) => o,
            ArtworkKind::Borrowed(b) => b,
        }
    }

    pub fn path_str(&self) -> &str {
        cstr(self.raw().path)
    }

    pub fn path(&self) -> &Path {
        Path::new(self.path_str())
    }
}

impl Drop for ArtworkInfo<'_> {
    fn drop(&mut self) {
        if let ArtworkKind::Owned(owned) = &mut self.kind {
            unsafe { sys::hue_free_artwork(owned) };
        }
    }
}
