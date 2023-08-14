#[derive(Clone, Debug, PartialEq)]
pub struct DomainName(pub Vec<String>);

impl DomainName {
    pub fn new(domain_as_labels: Vec<String>) -> DomainName {
        Self(domain_as_labels)
    }
}

impl From<String> for DomainName {
    fn from(name: String) -> Self {
        Self(name.split('.').map(String::from).collect())
    }
}

impl Into<String> for DomainName {
    fn into(self) -> String {
        self.0.join(".")
    }
}

impl From<DomainName> for Vec<u8> {
    fn from(name: DomainName) -> Vec<u8> {
        name.0.iter().flat_map(|s| s.as_bytes().to_vec()).collect()
    }
}

impl From<Vec<u8>> for DomainName {
    fn from(name: Vec<u8>) -> Self {
        Self(
            String::from_utf8(name)
                .unwrap()
                .split('.')
                .map(String::from)
                .collect()
        )
    }
}