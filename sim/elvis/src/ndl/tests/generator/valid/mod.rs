//! Tests on the NDL generator, for valid tests
#![cfg(test)]

use crate::ndl::generate_and_run_sim;

#[tokio::test]
#[ntest::timeout(100)]
async fn message() {
    generate_and_run_sim(include_str!("message.ndl")).await;
}

#[tokio::test]
#[ntest::timeout(100)]
async fn forward() {
    generate_and_run_sim(include_str!("forward.ndl")).await;
}

#[tokio::test]
#[ntest::timeout(100)]
async fn pingpong() {
    generate_and_run_sim(include_str!("pingpong.ndl")).await;
}

#[tokio::test]
#[ntest::timeout(100)]
async fn message_ip() {
    generate_and_run_sim(include_str!("message_ip.ndl")).await;
}

#[tokio::test]
#[ntest::timeout(100)]
async fn forward_ip() {
    generate_and_run_sim(include_str!("forward_ip.ndl")).await;
}

#[tokio::test]
#[ntest::timeout(100)]
async fn pingpong_ip() {
    generate_and_run_sim(include_str!("pingpong_ip.ndl")).await;
}
