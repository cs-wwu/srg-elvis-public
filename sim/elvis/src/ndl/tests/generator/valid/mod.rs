//! Tests on the NDL generator, for valid tests
#![cfg(test)]

use crate::ndl::generate_and_run_sim;

#[tokio::test]
#[ntest::timeout(100)]
async fn message() {
    generate_and_run_sim(include_str!("message.txt")).await;
}

#[tokio::test]
#[ntest::timeout(100)]
async fn basic_forward_valid_test() {
    generate_and_run_sim(include_str!("forward.txt")).await;
}

#[tokio::test]
#[ntest::timeout(100)]
async fn basic_pingpong_valid_test() {
    generate_and_run_sim(include_str!("pingpong.txt")).await;
}

#[tokio::test]
#[ntest::timeout(100)]
async fn basic_message_ip_valid_test() {
    generate_and_run_sim(include_str!("message_ip.txt")).await;
}

#[tokio::test]
#[ntest::timeout(100)]
async fn basic_forward_ip_valid_test() {
    generate_and_run_sim(include_str!("forward_ip.txt")).await;
}

#[tokio::test]
#[ntest::timeout(100)]
async fn basic_pingpong_ip_valid_test() {
    generate_and_run_sim(include_str!("pingpong_ip.txt")).await;
}
