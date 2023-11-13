use std::{sync::Arc, time::Duration};

use elvis_core::{
    new_machine_arc,
    protocols::{
        ipv4::{Ipv4Address, Recipient},
        dhcp::{DhcpClient, DhcpMessage},
        Arp, Endpoint, Endpoints, Ipv4, Pci, Udp,
    },
    run_internet, run_internet_with_timeout, ExitStatus, IpTable, Machine, Message, Network,
};

//todo: ping pong & send message with address resolution