use crate::applications::{Capture, SendMessage};
use elvis_core::{
    new_machine,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient},
        Endpoint, Pci, Udp,
    },
    run_internet, IpTable, Message, Network,
};

pub async fn localhost() {
    let network = Network::basic();
    let ip_address: Ipv4Address = [123, 45, 67, 89].into();
    let remote_endpoint: Endpoint = Endpoint::new(Ipv4Address::LOCALHOST, 0);

    let ip_table: IpTable<Recipient> = [("0.0.0.0/0", Recipient::new(0, None))]
        .into_iter()
        .collect();

    let machines = vec![new_machine![
        Udp::new(),
        Ipv4::new(ip_table.clone()),
        Pci::new([network.clone()]),
        SendMessage::new(vec![Message::new("Hello!")], remote_endpoint).local_ip(ip_address),
        Capture::new(remote_endpoint, 1),
    ]];

    run_internet(&machines).await;
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    #[tracing_test::traced_test]
    async fn localhost() {
        super::localhost().await;
    }
}
