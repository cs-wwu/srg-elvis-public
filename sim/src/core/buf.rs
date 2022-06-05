use std::rc::Rc;
use std::ops::{Range, RangeFull};

/// A Buf is a reference counted contiguous block of memory.
/// The struct holds a ref counted data pointer into the heap.
/// It also holds a start index, and a length
#[derive(Debug)]
pub struct Buf {
    data: Rc<Vec<u8>>,        // The actual data
    start: usize,             // The starting index
    length: usize,            // How many bytes
}

impl Buf {
    /// Create a new buf from the array
    ///
    /// # Example
    ///
    ///  let buf = Buf::new(b"Hello World");
    ///
    /// # Returns
    ///
    /// A new Buf structure
    pub fn new(bytes: &[u8]) -> Buf {
        Buf {
            data: Rc::new(bytes.to_vec()),
            start: 0,
            length: bytes.len()
        }
    }

    /// Create a new buf sliced from the original. This is a zero copy implementation
    ///
    /// # Arguments
    ///
    /// * `from` - The index to copy from
    /// * `to` - The index to stop at (exclusive)
    ///
    /// # Returns
    ///
    /// The new Buf with a cloned data pointer and updated indices
    pub fn slice(&self, from: usize, to: usize) -> Buf {
        assert!(from < self.start + self.length);
        assert!(to <= self.start + self.length);
        assert!(from <= to);
        Buf {
            data: Rc::clone(&self.data),
            start: self.start + from,
            length: to - from
        }
    }

    /// Return the length in bytes of this Buf
    pub fn len(&self) -> usize {
        self.length
    }

    /// Return true if the Buf is empty
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }
}

impl std::clone::Clone for Buf {
    /// Clone the buf. This clones the underlying ref counted buffer
    ///
    /// # Returns
    ///
    /// The new Buf that points to the same underlying ref counted data
    fn clone(&self) -> Buf {
        Buf {
            data: Rc::clone(&self.data),
            start: self.start,
            length: self.length
        }
    }
}

/// This implements index for the Buf
impl std::ops::Index<usize> for Buf {
    type Output = u8;

    /// Return the byte value at index i for the buf
    ///
    /// # Arguments
    ///
    /// * `idx` - The index of the value to get, displaced from self.start
    ///
    /// # Returns
    ///
    /// The byte value at index idx.
    fn index(&self, idx: usize) -> &Self::Output {
        assert!(idx < self.length);
        &self.data[self.start + idx]
    }
}

/// This implements index range for a buf `buf[start..end]`
impl std::ops::Index<Range<usize>> for Buf {
     type Output = [u8];

     /// Return the array of bytes in the given range
     ///
     /// # Arguments
     ///
     /// * `r` - The range for the data
     ///
     /// # Returns
     ///
     /// A reference to the array of data
     fn index(&self, r: Range<usize>) -> &Self::Output {
         assert!(self.start + self.length < r.start);
         assert!(self.start + self.length < r.end);
         &self.data[self.start + r.start .. self.start + r.end]
     }
}

/// This implements RangeFull for a buf `buf[..]`
impl std::ops::Index<RangeFull> for Buf {
     type Output = [u8];

     /// Return the array of bytes in the given range
     ///
     /// # Arguments
     ///
     /// * `r` - The range for the data
     ///
     /// # Returns
     ///
     /// A reference to the array of data
     fn index(&self, _r: RangeFull) -> &Self::Output {
         &self.data[self.start .. self.start + self.length]
     }
}
