use rustc_hash::FxHashMap;
use std::any::{Any, TypeId};

#[derive(Debug, Default)]
pub struct Control {
    inner: FxHashMap<TypeId, Box<dyn Any>>,
}

impl Control {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert<T: 'static>(&mut self, t: T) {
        self.inner.insert(TypeId::of::<T>(), Box::new(t));
    }

    pub fn get<T: 'static>(&self) -> Option<&T> {
        self.inner
            .get(&TypeId::of::<T>())
            .map(|t| t.downcast_ref())
            .flatten()
    }
}
