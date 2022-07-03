use thiserror::Error as ThisError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProtocolId {
    pub layer: NetworkLayer,
    // Todo: If there are many user programs, this should probably be a larger primitive
    pub identifier: u8,
}

impl ProtocolId {
    pub const fn new(layer: NetworkLayer, identifier: u8) -> Self {
        Self { layer, identifier }
    }

    pub fn to_bytes(self) -> [u8; 2] {
        <[u8; 2]>::from(self)
    }
}

impl From<ProtocolId> for [u8; 2] {
    fn from(id: ProtocolId) -> Self {
        [id.layer as u8, id.identifier]
    }
}

impl From<ProtocolId> for u16 {
    fn from(id: ProtocolId) -> Self {
        let bytes: [u8; 2] = id.into();
        Self::from_be_bytes(bytes)
    }
}

impl TryFrom<[u8; 2]> for ProtocolId {
    type Error = NetworkLayerError;

    fn try_from(value: [u8; 2]) -> Result<Self, Self::Error> {
        Ok(Self {
            layer: value[0].try_into()?,
            identifier: value[1],
        })
    }
}

impl TryFrom<u16> for ProtocolId {
    type Error = NetworkLayerError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        value.to_be_bytes().try_into()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum NetworkLayer {
    Link,
    Network,
    Transport,
    Application,
    User,
}

impl TryFrom<u8> for NetworkLayer {
    type Error = NetworkLayerError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(NetworkLayer::Link),
            1 => Ok(NetworkLayer::Network),
            2 => Ok(NetworkLayer::Transport),
            3 => Ok(NetworkLayer::Application),
            4 => Ok(NetworkLayer::User),
            _ => Err(NetworkLayerError::FromByte(value)),
        }
    }
}

#[derive(Debug, ThisError)]
pub enum NetworkLayerError {
    #[error("Unable to create a network layer from the byte {0}")]
    FromByte(u8),
}
