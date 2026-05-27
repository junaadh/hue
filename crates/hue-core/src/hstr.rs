use core::fmt;

const HUE_STRING_LEN: usize = 64;
pub type HString = HStr<HUE_STRING_LEN>;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct HStr<const CAP: usize> {
    buf: [u8; CAP],
    len: u8,
}

const fn assert_cap_u8<const CAP: usize>() {
    assert!(CAP <= u8::MAX as usize)
}

impl<const CAP: usize> HStr<CAP> {
    #[inline]
    pub const fn new() -> Self {
        assert_cap_u8::<CAP>();

        Self {
            buf: [0; CAP],
            len: 0,
        }
    }

    #[allow(clippy::should_implement_trait)]
    #[inline]
    pub fn from_str(value: &str) -> Self {
        let mut out = Self::new();
        out.truncate(value.as_bytes());
        out
    }

    #[inline]
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut out = Self::new();
        out.truncate(bytes);
        out
    }

    #[inline]
    pub fn truncate(&mut self, bytes: &[u8]) {
        let len_or_cap = bytes.len().min(CAP);
        let len = core::str::from_utf8(&bytes[..len_or_cap])
            .map_or_else(|err| err.valid_up_to(), |_| len_or_cap);

        self.buf[..len].copy_from_slice(&bytes[..len]);
        self.len = len as u8;
    }

    #[inline]
    pub fn try_from_str(value: &str) -> Result<Self, HStrError> {
        let mut out = Self::new();
        out.set(value.as_bytes())?;
        Ok(out)
    }

    #[inline]
    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, HStrError> {
        let mut out = Self::new();
        out.set(bytes)?;
        Ok(out)
    }

    #[inline]
    pub fn set(&mut self, bytes: &[u8]) -> Result<(), HStrError> {
        if bytes.len() > CAP || bytes.len() > u8::MAX as usize {
            return Err(HStrError::TooLong);
        }

        core::str::from_utf8(bytes).map_err(|_| HStrError::UTF8Error)?;

        self.buf[..bytes.len()].copy_from_slice(bytes);
        self.len = bytes.len() as u8;

        Ok(())
    }

    #[inline]
    pub const fn clear(&mut self) {
        self.len = 0;
    }

    #[inline]
    pub const fn len(&self) -> usize {
        self.len as usize
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf[..self.len()]
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        unsafe { core::str::from_utf8_unchecked(self.as_bytes()) }
    }
}

impl<const CAP: usize> Default for HStr<CAP> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const CAP: usize> fmt::Debug for HStr<CAP> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("HStr").field(&self.as_str()).finish()
    }
}

impl<const CAP: usize> fmt::Display for HStr<CAP> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HStrError {
    UTF8Error,
    TooLong,
}

impl fmt::Display for HStrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::UTF8Error => "utf8 error",
                Self::TooLong => "too long",
            }
        )
    }
}

impl core::error::Error for HStrError {}
