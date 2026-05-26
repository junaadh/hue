use crate::{cstr, sys};

#[allow(dead_code)]
pub(crate) enum SongKind<'a> {
    Owned(sys::HueSongInfoRaw),
    Borrowed(&'a sys::HueSongInfoRaw),
}

pub struct SongInfo<'a> {
    pub(crate) kind: SongKind<'a>,
}

impl SongInfo<'_> {
    #[inline(always)]
    const fn raw(&self) -> &sys::HueSongInfoRaw {
        match &self.kind {
            SongKind::Owned(o) => o,
            SongKind::Borrowed(b) => b,
        }
    }

    pub fn title(&self) -> &str {
        cstr(self.raw().title)
    }

    pub fn artist(&self) -> &str {
        cstr(self.raw().artist)
    }

    pub fn album(&self) -> &str {
        cstr(self.raw().album)
    }

    pub fn app_name(&self) -> &str {
        cstr(self.raw().app_name)
    }
}
