#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Checksum {
    sum: u16,
    carry: bool,
}

impl Checksum {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_u16(&mut self, value: u16) {
        (self.sum, self.carry) = self.sum.carrying_add(value, self.carry);
    }

    pub fn add_u8(&mut self, a: u8, b: u8) {
        self.add_u16(u16::from_be_bytes([a, b]));
    }

    pub fn add_u32(&mut self, value: [u8; 4]) {
        self.add_u8(value[0], value[1]);
        self.add_u8(value[2], value[3]);
    }

    pub fn as_u16(&self) -> u16 {
        match self.sum {
            // Use that there are two one's complement representations of zero
            // and pick the nonzero one to differentiate from an unused
            // checksum.
            0xffff => 0xffff,
            sum => !sum,
        }
    }
}
