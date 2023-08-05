pub struct DomainName(Vec<String>);

impl DomainName {
    pub const ROOT: Self = Self(["."]);

    pub fn new(domain_as_labels: Vec<String>) -> DomainName {
        Self(domain_as_labels)
    }
}

impl From<String> for DomainName {
    fn from(name: String) -> Self {
        Self = (name.split('.').map(String::from).collect())
    }
}

impl From<DomainName> for Vec<u8> {
    fn from(name: DomainName) -> Vec<u8> {

    }
}