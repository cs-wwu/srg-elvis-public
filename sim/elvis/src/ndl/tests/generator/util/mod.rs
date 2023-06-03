//! Tests on generator util functions
#![cfg(test)]

use crate::ndl::generate_and_run_sim;

#[tokio::test]
#[should_panic(
    expected = "assertion failed: `(left == right)`\n  left: `5`,\n right: `4`: Network 1: Invalid IP octect count, expected 4 octets found 5 octets"
)]
async fn generator_util_invalid_ip_octets() {
    generate_and_run_sim(include_str!("invalid_ip_octets.txt")).await;
}

#[tokio::test]
#[should_panic(expected = "Invalid IP octet expected u8. In Network 1, found: 99999")]
async fn generator_util_invalid_ip_u8_octets() {
    generate_and_run_sim(include_str!("invalid_ip_u8_octets.txt")).await;
}

#[tokio::test]
#[should_panic(expected = "Port declaration error. Found port: beefasdasd")]
async fn generator_util_invalid_port_number() {
    generate_and_run_sim(include_str!("invalid_port_number.txt")).await;
}
