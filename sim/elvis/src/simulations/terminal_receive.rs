use crate::applications::Terminal;
use elvis_core::{
    new_machine_arc,
    protocols::{
        ipv4::{Ipv4, Recipient},
        udp::Udp,
        Endpoint, Pci,
    },
    ExitStatus, IpTable, Network, run_internet,
};

/*
* Usage:
*               # Start the sim #
*
*   $cargo test terminal_receive -- --nocapture
*       output: Begin run on port 127.0.0.1:[portnumber]
*               Begin run on port 127.0.0.1:[other portnumber]
*
*         # Open two new terminals and connect to machines #
*
*   $telnet localhost [portnumber]
*   $telnet localhost [other portnumber]
*       output: Connected to localhost.
*               Escape character is '^]'.
*
*   # Send message between terminals #
*
*   $send [endpoint] message 
*       note: [endpoints] are commented next to the terminal receive params
*/
pub async fn terminal_receive() {
    let network = Network::basic();
    let endpoint = Endpoint {
        // endpoint: 123.45.67.89:48879
        address: [123, 45, 67, 89].into(),
        port: 0xbeef, // 48879
    };
    let local = Endpoint {
        // endpoint: 123.44.66.88:65261
        address: [123, 44, 66, 88].into(),
        port: 0xfeed, // 65261
    };

    let ip_table: IpTable<Recipient> = [(local.address, Recipient::with_mac(0, 1)), (endpoint.address, Recipient::with_mac(0, 0))]
        .into_iter()
        .collect();

    let machines = vec![
        new_machine_arc![
            Udp::new(),
            Ipv4::new(ip_table.clone()),
            Pci::new([network.clone()]),
            Terminal::new(local, String::from("localhost:0")),
        ],
        new_machine_arc![
            Udp::new(),
            Ipv4::new(Default::default()),
            Pci::new([network.clone()]),
            Terminal::new(endpoint, String::from("localhost:0")),
        ],
    ];

    let status = run_internet(&machines, None).await;
    assert_eq!(status, ExitStatus::Exited);
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    pub async fn terminal_receive() {
        super::terminal_receive().await;
    }
}
