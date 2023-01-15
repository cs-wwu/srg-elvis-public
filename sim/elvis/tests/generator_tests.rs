//! Main generator tests
use elvis::ndl::generate_sim;

/// Catches an intended panic
/// used for when a test should fail as intended
fn catch_panic() {
    std::panic::set_hook(Box::new(|_info| {
        // do nothing
    }));
}

#[tokio::test]
#[should_panic(
    expected = "assertion failed: `(left == right)`\n  left: `5`,\n right: `4`: Network 1: Invalid IP octect count, expected 4 octets found 5 octets"
)]
async fn generator_invalid_ip_octets() {
    catch_panic();
    let file_path: String = "./tests/generator_tests/invalid_ip_octets.txt".to_string();
    generate_sim(file_path).await;
}

#[tokio::test]
#[should_panic(expected = "Invalid IP octet expected u8. In Network 1, found: 99999")]
async fn generator_invalid_ip_u8_octets() {
    catch_panic();
    let file_path: String = "./tests/generator_tests/invalid_ip_u8_octets.txt".to_string();
    generate_sim(file_path).await;
}

#[tokio::test]
#[should_panic(expected = "Invalid Network ID found. Got 89 expected \"1 , 5\"")]
async fn generator_invalid_network_id() {
    catch_panic();
    let file_path: String = "./tests/generator_tests/invalid_network_id.txt".to_string();
    generate_sim(file_path).await;
}

#[tokio::test]
#[should_panic(expected = "Invalid Protocol found in machine. Found: BadProtocol")]
async fn generator_invalid_protocol() {
    catch_panic();
    let file_path: String = "./tests/generator_tests/invalid_protocol.txt".to_string();
    generate_sim(file_path).await;
}
