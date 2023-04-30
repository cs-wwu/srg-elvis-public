#![allow(unused)]

use super::{
    ipv4_parsing::{ControlFlags, Ipv4Header, TypeOfService},
    Ipv4Address,
};

pub struct TestHeaderBuilder {
    total_length: u16,
    flags: ControlFlags,
    identification: u16,
    fragment_offset: u16,
    may_fragment: bool,
    last_fragment: bool,
}

impl TestHeaderBuilder {
    pub const fn new(total_length: u16) -> Self {
        Self {
            total_length,
            flags: ControlFlags::DEFAULT,
            identification: 1337,
            fragment_offset: 0,
            may_fragment: true,
            last_fragment: true,
        }
    }

    pub const fn with_message_len(message_len: u16) -> Self {
        Self::new(message_len + 20)
    }

    pub const fn dont_fragment(mut self) -> Self {
        self.may_fragment = false;
        self
    }

    pub const fn more_fragments(mut self) -> Self {
        self.last_fragment = false;
        self
    }

    pub const fn identification(mut self, identification: u16) -> Self {
        self.identification = identification;
        self
    }

    pub const fn fragment_offset(mut self, offset_bytes: u16) -> Self {
        self.fragment_offset = offset_bytes / 8;
        self
    }

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
