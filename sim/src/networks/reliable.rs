use crate::core::Network;

/// A network maximum transmission unit.
///
/// The largest number of bytes that can be sent over the network at once.
pub type Mtu = u32;

pub struct Reliable {
    // TODO(hardint): Add a way to access the MTU by other protocols
    // TODO(hardint): Only allow messages up to `mtu` in size
    /// The maximum transmission unit of the network
    #[allow(dead_code)]
    mtu: Mtu,
}

impl Reliable {
    pub fn new(mtu: Mtu) -> Self {
        Self { mtu }
    }
}

impl Network for Reliable {
    fn send(
        self: std::sync::Arc<Self>,
        delivery: crate::protocols::tap::Delivery,
        attachments: &[crate::core::network::Attachment],
    ) -> Result<(), Box<dyn std::error::Error>> {
        todo!()
    }
}
