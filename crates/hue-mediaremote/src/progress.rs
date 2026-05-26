use crate::sys;

pub(crate) enum ProgressKind<'a> {
    Owned(sys::HueProgressInfoRaw),
    Borrowed(&'a sys::HueProgressInfoRaw),
}

pub struct ProgressInfo<'a> {
    pub(crate) kind: ProgressKind<'a>,
}

impl ProgressInfo<'_> {
    #[inline(always)]
    const fn raw(&self) -> &sys::HueProgressInfoRaw {
        match &self.kind {
            ProgressKind::Owned(o) => o,
            ProgressKind::Borrowed(b) => b,
        }
    }

    pub fn position(&self) -> f64 {
        self.raw().position
    }

    pub fn duration(&self) -> f64 {
        self.raw().duration
    }
}
