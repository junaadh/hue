#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeqStatus {
    Expected,
    Gap { expected: u16, received: u16 },
    Duplicate { expected: u16, received: u16 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeqTracker {
    expected: u16,
}

impl SeqTracker {
    pub const fn new(initial: u16) -> Self {
        Self { expected: initial }
    }

    pub const fn expected(&self) -> u16 {
        self.expected
    }

    pub fn observe(&mut self, received: u16) -> SeqStatus {
        match seq_distance(received, self.expected) {
            0 => {
                self.expected = self.expected.wrapping_add(1);
                SeqStatus::Expected
            }
            distance if distance > 0 => SeqStatus::Gap {
                expected: self.expected,
                received,
            },
            _ => SeqStatus::Duplicate {
                expected: self.expected,
                received,
            },
        }
    }
}

#[inline]
pub fn seq_distance(received: u16, expected: u16) -> i16 {
    received.wrapping_sub(expected) as i16
}
