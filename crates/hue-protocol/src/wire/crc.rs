const POLY: u16 = 0x1021;
const INIT: u16 = 0xFFFF;

const fn make_crc_table() -> [Crc; 256] {
    let mut table = [Crc::new(0); 256];
    let mut i = 0usize;

    while i < 256 {
        let mut crc = (i as u16) << 8;
        let mut j = 0;

        while j < 8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ POLY
            } else {
                crc << 1
            };

            j += 1;
        }

        table[i] = Crc::new(crc);
        i += 1;
    }

    table
}

static CRC_TABLE: [Crc; 256] = make_crc_table();

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Crc(u16);

impl Crc {
    #[inline]
    pub const fn new(raw: u16) -> Self {
        Self(raw)
    }

    #[inline]
    pub const fn raw(self) -> u16 {
        self.0
    }

    pub const INIT: Self = Crc::new(INIT);

    pub fn feed(&mut self, data: &[u8]) {
        for &byte in data {
            let idx = (((self.0 >> 8) ^ byte as u16) & 0xff) as usize;
            *self = Crc::new((self.0 << 8) ^ CRC_TABLE[idx].0)
        }
    }
}
