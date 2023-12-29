//! An implementation of [Internet Group Managemet
//! Protocol](https://datatracker.ietf.org/doc/html/rfc1112)

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct MulticastAddress([u8; 4]);

impl MulticastAddress{
    pub fn new(address: [u8; 4]) -> Self {
        // Check the high-order four bits are "1110"
        if (address[0] >> 4) != 0b1110 {
            panic!("Invalid multicast address")
        }
        Self(address)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_addr() {
        let addr: MulticastAddress = MulticastAddress::new([224,0,0,1]);
        assert_eq!(addr, addr);
    }
    #[test]
    #[should_panic]
    fn init_bad_addr() {
        let _addr1: MulticastAddress = MulticastAddress::new([192,0,0,1]);
    }


}
