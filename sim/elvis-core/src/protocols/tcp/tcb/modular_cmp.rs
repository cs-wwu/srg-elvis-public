pub use ModCmp::*;

/// Is a < b under modular arithmetic?
pub fn mod_lt(a: u32, b: u32) -> bool {
    // k is on the opposite side of the ring of integers mod 32 from b
    let k = b.wrapping_add(u32::MAX / 2);

    // There are six cases:
    //  0123456789
    // |a b    k  | a<b, a<k, b<k -> a<b
    // |a k    b  | a<b, a<k, b>k -> a>b
    // |  b a  k  | a>b, a<k, b<k -> a>b
    // |  k a  b  | a<b, a>k, b>k -> a<b
    // |  b    k a| a>b, a>k, b<k -> a<b
    // |  k    b a| a>b, a>k, b>k -> a>b

    (a < b) ^ (a < k) ^ (b < k)
}

/// Is a <= b under modular arithmetic?
pub fn mod_leq(a: u32, b: u32) -> bool {
    mod_lt(a, b.wrapping_add(1))
}

/// Is a > b under modular arithmetic?
pub fn mod_gt(a: u32, b: u32) -> bool {
    mod_lt(b, a)
}

/// Is a > b under modular arithmetic?
pub fn mod_geq(a: u32, b: u32) -> bool {
    mod_lt(b.wrapping_sub(1), a)
}

/// Is `b` between `a` and `c` when accounting for modular arithmetic?
pub fn mod_bounded(a: u32, ab_cmp: ModCmp, b: u32, bc_cmp: ModCmp, c: u32) -> bool {
    let a = a.wrapping_sub(ab_cmp.offset());
    let c = c.wrapping_add(bc_cmp.offset());

    // a < b < c holds under the following conditions:
    // j: | a b c |
    // k: | c a b |
    // l: | b c a |

    let j = a < b && b < c && a < c;
    let k = a < b && b > c && a > c;
    let l = a > b && b < c && a > c;
    j || k || l
}

/// Comparison options for [`mod_bounded`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModCmp {
    /// Less than
    Lt,
    /// Less than or equal to
    Leq,
}

impl ModCmp {
    /// How much to offset one of the bounds to convert a less than comparison
    /// to the given comparison
    fn offset(self) -> u32 {
        match self {
            Lt => 0,
            Leq => 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modular_comparison() {
        // 2**31 = 2_147_483_648
        assert!(mod_lt(10, 20));
        assert!(!mod_lt(20, 10));
        assert!(mod_lt(2_000_000_000, 3_000_000_000));
        assert!(!mod_lt(3_000_000_000, 2_000_000_000));
        assert!(mod_lt(3_000_000_000, 4_000_000_000));
        assert!(!mod_lt(4_000_000_000, 3_000_000_000));

        assert!(!mod_lt(5, 5));
        assert!(mod_leq(5, 5));

        assert!(mod_gt(20, 10));
        assert!(!mod_gt(5, 5));
        assert!(mod_geq(5, 5));

        assert!(mod_bounded(5, Lt, 10, Lt, 15));
        assert!(!mod_bounded(15, Lt, 10, Lt, 5));

        assert!(mod_bounded(u32::MAX - 5, Lt, 5, Lt, 10));
        assert!(!mod_bounded(10, Lt, 5, Lt, u32::MAX - 5));

        assert!(mod_bounded(u32::MAX - 10, Lt, u32::MAX - 5, Lt, 5));
        assert!(!mod_bounded(5, Lt, u32::MAX - 5, Lt, u32::MAX - 10));

        assert!(!mod_bounded(5, Lt, 5, Lt, 15));
        assert!(mod_bounded(5, Leq, 5, Lt, 15));
        assert!(!mod_bounded(5, Lt, 15, Lt, 15));
        assert!(mod_bounded(5, Lt, 15, Leq, 15));
        assert!(mod_bounded(10, Leq, 10, Leq, 10));
    }
}
