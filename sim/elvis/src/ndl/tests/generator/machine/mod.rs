//! Tests on generator machine functions
#![cfg(test)]

use crate::ndl::generate_and_run_sim;

#[tokio::test]
#[should_panic(expected = "Invalid Protocol found in machine. Found: BadProtocol")]
async fn generator_machine_invalid_protocol() {
    generate_and_run_sim(include_str!("invalid_protocol.txt")).await;
}

#[tokio::test]
#[should_panic(expected = "No ID found in network being added to machine.")]
async fn generator_machine_invalid_network() {
    generate_and_run_sim(include_str!("invalid_network.txt")).await;
}

#[tokio::test]
#[should_panic(expected = "Invalid Network ID found. Got 89 expected \"1 , 5\"")]
async fn generator_machine_invalid_network_id() {
    generate_and_run_sim(include_str!("invalid_network_id.txt")).await;
}

#[tokio::test]
#[should_panic(expected = "Invalid application in machine. Got application bad_application")]
async fn generator_machine_invalid_application_name() {
    generate_and_run_sim(include_str!("invalid_application_name.txt")).await;
}

#[tokio::test]
#[should_panic(expected = "Invalid name for 'to' in send_message, found: pizza")]
async fn generator_machine_invalid_machine_name_in_application() {
    generate_and_run_sim(include_str!("invalid_machine_name.txt")).await;
}

#[tokio::test]
#[should_panic(expected = "Capture application doesn't contain message_count.")]
async fn generator_machine_missing_message_count_in_capture() {
    generate_and_run_sim(include_str!("missing_message_count_in_capture.txt")).await;
}

#[tokio::test]
#[should_panic(expected = "Machine application does not contain a name")]
async fn generator_machine_missing_application_name() {
    generate_and_run_sim(include_str!("missing_application_name.txt")).await;
}
