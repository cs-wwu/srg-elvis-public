//! Implements the reassembly procedure from RFC791, section 3.2, page 27: An Example
//! Reassembly Procedure
//! https://www.rfc-editor.org/rfc/rfc791

#[derive(Debug, Clone, PartialEq, Eq)]
struct BitSet {
    bits: Vec<u8>,
}

impl BitSet {
    fn new(bits: usize) -> Self {
        Self {
            bits: vec![0u8; (bits - 1) / 8 + 1],
        }
    }

    fn get(&self, bit: usize) -> bool {
        ((self.bits[bit / 8] >> (bit % 8)) & 1) == 1
    }

    fn set(&mut self, bit: usize) {
        self.bits[bit / 8] |= 1 << (bit % 8);
    }

    fn set_range(&mut self, start: usize, end: usize) {
        for i in start..end {
            self.set(i)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bit_set() {
        let mut bits = BitSet::new(100);
        bits.set_range(10, 50);
        assert!(!bits.get(5));
        assert!(bits.get(10));
        assert!(bits.get(30));
        assert!(bits.get(49));
        assert!(!bits.get(50));
        assert!(!bits.get(75));
    }
}
