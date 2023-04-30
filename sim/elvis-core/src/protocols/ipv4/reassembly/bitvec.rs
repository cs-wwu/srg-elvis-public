/// A vector of individual bits for space efficiency.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BitVec {
    bits: Vec<u8>,
}

impl BitVec {
    /// Creates a bit vector of the given length
    pub fn new() -> Self {
        Default::default()
    }

    /// Whether the given bit is set
    pub fn get(&self, bit: u16) -> bool {
        match self.bits.get(bit as usize / 8) {
            Some(byte) => ((byte >> (bit % 8)) & 1) == 1,
            None => false,
        }
    }

    /// Sets the given bit high
    pub fn set(&mut self, bit: u16) {
        let byte = bit as usize / 8;
        self.bits.resize(self.bits.len().max(byte + 1), 0);
        self.bits[byte] |= 1 << (bit % 8);
    }

    /// Sets the range of bits from start (inclusive) to end (exclusive) high.
    pub fn set_range(&mut self, start: u16, end: u16) {
        // TODO(hardint): Can be made more efficient by setting entire u8 blocks
        // instead of individual bits
        for i in start..end {
            self.set(i)
        }
    }

    /// Checks whether all the bits in the vector have been set high.
    pub fn complete(&self, len: u16) -> bool {
        // TODO(hardint): Can be made more efficient by checking entire u8 blocks
        // instead of individual bits
        (0..len).all(|i| self.get(i))
    }

    pub fn count(&self) -> u16 {
        (0u16..).find(|i| !self.get(*i)).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bit_set() {
        let mut bits = BitVec::new();
        bits.set_range(10, 50);
        assert!(!bits.get(5));
        assert!(bits.get(10));
        assert!(bits.get(30));
        assert!(bits.get(49));
        assert!(!bits.get(50));
        assert!(!bits.get(75));
    }
}
