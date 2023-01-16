//! Tests on generator network functions
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
    expected = "assertion failed: `(left == right)`\n  left: `1`,\n right: `2`: Network 1: Invalid IP range format, expected 2 values found 1"
)]
async fn generator_network_invalid_ip_range_format() {
    catch_panic();
    let file_path: String =
        "./tests/generator_tests/networks/invalid_ip_range_format.txt".to_string();
    generate_sim(file_path).await;
}

#[tokio::test]
#[should_panic(expected = "Network 1: Invalid ending IP range number. Expected <u8> found: 90000")]
async fn generator_network_invalid_ending_ip_range() {
    catch_panic();
    let file_path: String =
        "./tests/generator_tests/networks/invalid_ending_ip_range.txt".to_string();
    generate_sim(file_path).await;
}

#[tokio::test]
#[should_panic(
    expected = "Network 1: Invalid Cidr format, end IP value (69) greater than start IP value (89)"
)]
async fn generator_network_invalid_ending_ip_value() {
    catch_panic();
    let file_path: String =
        "./tests/generator_tests/networks/invalid_ending_ip_value.txt".to_string();
    generate_sim(file_path).await;
}

#[tokio::test]
#[should_panic(expected = "Network 5: Duplicate IP found in range: [12, 34, 56, 89]")]
async fn generator_network_duplicate_ip_range() {
    catch_panic();
    let file_path: String = "./tests/generator_tests/networks/duplicate_ip_range.txt".to_string();
    generate_sim(file_path).await;
}

#[tokio::test]
#[should_panic(expected = "Network 5: Duplicate IP found in IP: [192, 168, 1, 121]")]
async fn generator_network_duplicate_ip() {
    catch_panic();
    let file_path: String = "./tests/generator_tests/networks/duplicate_ip.txt".to_string();
    generate_sim(file_path).await;
}

#[tokio::test]
#[should_panic(expected = "Network 5: Invalid network argument provided. Found: badargument")]
async fn generator_network_invalid_network_argument() {
    catch_panic();
    let file_path: String =
        "./tests/generator_tests/networks/invalid_network_argument.txt".to_string();
    generate_sim(file_path).await;
}
