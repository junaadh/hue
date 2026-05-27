pub const FNV1A32_INIT: u32 = 0x811C_9DC5;
pub const FNV1A32_PRIME: u32 = 0x0100_0193;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fnv32(u32);

impl Fnv32 {
    #[inline]
    pub const fn new() -> Self {
        Self(FNV1A32_INIT)
    }

    #[inline]
    pub const fn finish(self) -> u32 {
        self.0
    }

    #[inline]
    pub fn feed_byte(&mut self, byte: u8) {
        self.0 ^= byte as u32;
        self.0 = self.0.wrapping_mul(FNV1A32_PRIME);
    }

    #[inline]
    pub fn feed_bytes(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.feed_byte(byte);
        }
    }

    #[inline]
    pub fn feed_str(&mut self, value: &str) {
        self.feed_bytes(value.as_bytes());
    }

    #[inline]
    pub fn feed_u64(&mut self, value: u64) {
        self.feed_bytes(&value.to_le_bytes());
    }

    #[inline]
    pub fn separator(&mut self) {
        self.feed_byte(0xFF);
    }
}

impl Default for Fnv32 {
    fn default() -> Self {
        Self::new()
    }
}

pub fn track_id_from_parts(
    title: &str,
    artist: &str,
    album: &str,
    duration_ms: u64,
) -> u32 {
    let mut hash = Fnv32::new();

    hash.feed_str(title);
    hash.separator();

    hash.feed_str(artist);
    hash.separator();

    hash.feed_str(album);
    hash.separator();

    hash.feed_u64(duration_ms);

    hash.finish()
}
