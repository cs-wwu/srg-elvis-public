/// A vector of individual bits for space efficiency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitVec {
    bits: Vec<u8>,
    len: u16,
}

impl BitVec {
    /// Creates a bit vector of the given length
    pub fn new(len: u16) -> Self {
        let bytes = (len - 1) / 8 + 1;
        Self {
            bits: vec![0u8; bytes as usize],
            len,
        }
    }

    /// Whether the given bit is set
    pub fn get(&self, bit: u16) -> bool {
        ((self.bits[bit as usize / 8] >> (bit % 8)) & 1) == 1
    }

    /// Sets the given bit high
    pub fn set(&mut self, bit: u16) {
        self.bits[bit as usize / 8] |= 1 << (bit % 8);
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
    pub fn complete(&self) -> bool {
        // TODO(hardint): Can be made more efficient by checking entire u8 blocks
        // instead of individual bits
        (0..self.len).all(|i| self.get(i))
    }

    pub fn len(&self) -> u16 {
        self.len
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bit_set() {
        let mut bits = BitVec::new(100);
        bits.set_range(10, 50);
        assert!(!bits.get(5));
        assert!(bits.get(10));
        assert!(bits.get(30));
        assert!(bits.get(49));
        assert!(!bits.get(50));
        assert!(!bits.get(75));
    }
}
