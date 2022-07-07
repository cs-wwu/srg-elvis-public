use std::hash::{Hash, Hasher};

/// Since we only work with static strings for [`Control`], we use a newtype to
/// make the string hash based on its pointer to speed up map performance.
#[derive(Debug, Clone, Copy)]
pub struct StaticStr(&'static str);

impl Hash for StaticStr {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.as_ptr().hash(state)
    }
}

impl From<&'static str> for StaticStr {
    fn from(s: &'static str) -> Self {
        Self(s)
    }
}

impl PartialEq for StaticStr {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_ptr() == other.0.as_ptr()
    }
}

impl Eq for StaticStr {}

#[cfg(test)]
mod tests {
    use std::{
        collections::hash_map::RandomState,
        hash::{BuildHasher, Hash, Hasher},
    };

    use super::StaticStr;

    #[test]
    fn static_str() {
        let s1 = StaticStr("Hi!");
        let s2 = StaticStr("Hi!");
        assert_eq!(s1, s2);

        let hasher_builder = RandomState::new();
        let hashes: Vec<_> = [s1, s2]
            .into_iter()
            .map(|s| {
                let mut hasher = hasher_builder.build_hasher();
                s.hash(&mut hasher);
                hasher.finish()
            })
            .collect();

        assert_eq!(hashes[0], hashes[1]);

        let s1 = StaticStr("John!");
        let s2 = StaticStr("Paul!");
        assert_ne!(s1, s2);

        let hasher_builder = RandomState::new();
        let hashes: Vec<_> = [s1, s2]
            .into_iter()
            .map(|s| {
                let mut hasher = hasher_builder.build_hasher();
                s.hash(&mut hasher);
                hasher.finish()
            })
            .collect();

        assert_ne!(hashes[0], hashes[1]);
    }
}
