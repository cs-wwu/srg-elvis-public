//! Tests on generator util functions
use elvis::ndl::generate_and_run_sim;

/// Catches an intended panic
/// used for when a test should fail as intended
fn catch_panic() {
    std::panic::set_hook(Box::new(|_info| {
        // do nothing
    }));
}

#[tokio::test(flavor = "multi_thread")]
#[should_panic(
    expected = "Network 1: Invalid IP octect count, expected 4 octets found 5 octets"
)]
async fn generator_util_invalid_ip_octets() {
    catch_panic();
    let file_path: String = "./tests/generator_tests/utils/invalid_ip_octets.txt".to_string();
    generate_and_run_sim(file_path).await;
}

#[tokio::test(flavor = "multi_thread")]
#[should_panic(expected = "Invalid IP octet expected u8. In Network 1, found: 99999")]
async fn generator_util_invalid_ip_u8_octets() {
    catch_panic();
    let file_path: String = "./tests/generator_tests/utils/invalid_ip_u8_octets.txt".to_string();
    generate_and_run_sim(file_path).await;
}

#[tokio::test(flavor = "multi_thread")]
#[should_panic(expected = "Port declaration error. Found port: beefasdasd")]
async fn generator_util_invalid_port_number() {
    catch_panic();
    let file_path: String = "./tests/generator_tests/utils/invalid_port_number.txt".to_string();
    generate_and_run_sim(file_path).await;
}
