#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UniqueKey {
    id: usize,
}

impl UniqueKey {
    pub fn new() -> Self {
        let id = Box::leak(Box::new(0u8)) as *const u8 as usize;
        Self { id }
    }
}
