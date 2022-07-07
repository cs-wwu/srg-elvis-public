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
