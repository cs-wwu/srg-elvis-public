use super::{
    ipv4_parsing::{ControlFlags, Ipv4Header, TypeOfService},
    Ipv4Address,
};

/// Simplies creating IP headers for unit tests.
#[allow(unused)]
pub struct TestHeaderBuilder {
    /// The total length IP header field
    total_length: u16,
    /// The identification IP header field
    identification: u16,
    /// The fragment offset IP header field
    fragment_offset: u16,
    /// The DF flag
    may_fragment: bool,
    /// The MF flag
    last_fragment: bool,
}

#[allow(unused)]
impl TestHeaderBuilder {
    /// Create a new header builder with sensible defaults
    pub const fn new(total_length: u16) -> Self {
        Self {
            total_length,
            identification: 1337,
            fragment_offset: 0,
            may_fragment: true,
            last_fragment: true,
        }
    }

    /// Adds the header size to the total length field
    pub const fn ihl(mut self) -> Self {
        self.total_length += 20;
        self
    }

    /// Set the DF flag to 1
    pub const fn dont_fragment(mut self) -> Self {
        self.may_fragment = false;
        self
    }

    /// Set the MF flag to 1
    pub const fn more_fragments(mut self) -> Self {
        self.last_fragment = false;
        self
    }

    /// Set the identification field
    pub const fn identification(mut self, identification: u16) -> Self {
        self.identification = identification;
        self
    }

    /// Set the fragment offset field
    pub const fn fragment_offset(mut self, offset_bytes: u16) -> Self {
        self.fragment_offset = offset_bytes / 8;
        self
    }

    /// Build the IP header
    pub const fn build(self) -> Ipv4Header {
        Ipv4Header {
            total_length: self.total_length,
            flags: ControlFlags::new(self.may_fragment, self.last_fragment),
            fragment_offset: self.fragment_offset,
            identification: self.identification,
            ihl: 5,
            type_of_service: TypeOfService::DEFAULT,
            time_to_live: 30,
            protocol: 17,
            checksum: 0,
            source: Ipv4Address::CURRENT_NETWORK,
            destination: Ipv4Address::CURRENT_NETWORK,
        }
    }
}
