use crate::FxDashMap;
use super::dns_parsing::DnsResourceRecord;

#[derive(Debug, Clone)]
/// A struct defining the cache present in every implementation of the DNS.
/// Mappings are composed of a key: [String], and a value: [DnsResourceRecord].
pub struct DnsCache {name_to_rr: FxDashMap<String, DnsResourceRecord>}

impl DnsCache {
     /// Creates a new instance of the protocol.
     pub fn new() -> Self {
        Self {
            name_to_rr: Default::default(),
        }
    }

    /// Adds a new mapping to the name_to_rr cache.
    pub fn add_mapping(&self, name: String, rr: DnsResourceRecord) {
        self.name_to_rr.insert(name, rr);
    }

    /// Checks local name_to_rr cache for ['Ipv4Address'] given a name.
    pub fn get_mapping(&self, name: &str) -> Result<DnsResourceRecord, DnsCacheError> {
        match self.name_to_rr.get(name) {
            Some(e) => Ok(e.to_owned()),
            None => Err(DnsCacheError::Cache),
        }
    }
}

#[derive(Debug, thiserror::Error, Clone, Copy, PartialEq, Eq)]
pub enum DnsCacheError {
    #[error("DNS cache lookup error")]
    Cache,
    #[error("Unspecified DNS error")]
    Other,
}