use std::time::Duration;

use crate::applications::{Capture, SendMessage};
use elvis_core::{
    message::Message,
    new_machine,
    protocols::{
        ipv4::{Ipv4, Ipv4Address, Recipient},
        udp::Udp,
        Endpoint, Pci,
    },
    run_internet_with_timeout, ExitStatus, IpTable, Network,
};

pub fn test_reassembly() {
    let messages = Vec::new();

    // create big message
    let mut message1 = String::with_capacity(4096);
    let word = "bingus";
    while message1.len() + word.len() < 4096 {
        message1.push_str(word);
    }

    let bob = new_machine![
        SendMessage::new()
    ];
}