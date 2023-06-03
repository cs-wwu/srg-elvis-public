#![cfg(test)]

//! Tests on generator network functions
use crate::ndl::generate_and_run_sim;

#[tokio::test]
#[should_panic(
    expected = "assertion failed: `(left == right)`\n  left: `1`,\n right: `2`: Network 1: Invalid IP range format, expected 2 values found 1"
)]
async fn invalid_ip_range_format() {
    generate_and_run_sim(include_str!("invalid_ip_range_format.ndl")).await;
}

#[tokio::test]
#[should_panic(expected = "Network 1: Invalid ending IP range number. Expected <u8> found: 90000")]
async fn invalid_ending_ip_range() {
    generate_and_run_sim(include_str!("invalid_ending_ip_range.ndl")).await;
}

#[tokio::test]
#[should_panic(
    expected = "Network 1: Invalid Cidr format, end IP value (69) greater than start IP value (89)"
)]
async fn invalid_ending_ip_value() {
    generate_and_run_sim(include_str!("invalid_ending_ip_value.ndl")).await;
}

#[tokio::test]
#[should_panic(expected = "Network 5: Duplicate IP found in range: [12, 34, 56, 89]")]
async fn duplicate_ip_range() {
    generate_and_run_sim(include_str!("duplicate_ip_range.ndl")).await;
}

#[tokio::test]
#[should_panic(expected = "Network 5: Duplicate IP found in IP: [192, 168, 1, 121]")]
async fn duplicate_ip() {
    generate_and_run_sim(include_str!("duplicate_ip.ndl")).await;
}

#[tokio::test]
#[should_panic(expected = "Network 5: Invalid network argument provided. Found: badargument")]
async fn invalid_network_argument() {
    generate_and_run_sim(include_str!("invalid_network_argument.ndl")).await;
}
