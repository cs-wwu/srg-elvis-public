//! Tests on generator machine functions
use elvis::ndl::generate_sim;

/// Catches an intended panic
/// used for when a test should fail as intended
fn catch_panic() {
    std::panic::set_hook(Box::new(|_info| {
        // do nothing
    }));
}

#[tokio::test]
#[should_panic(expected = "Invalid Protocol found in machine. Found: BadProtocol")]
async fn generator_machine_invalid_protocol() {
    catch_panic();
    let file_path: String = "./tests/generator_tests/machines/invalid_protocol.txt".to_string();
    generate_sim(file_path).await;
}

#[tokio::test]
#[should_panic(expected = "No ID found in network being added to machine.")]
async fn generator_machine_invalid_network() {
    catch_panic();
    let file_path: String = "./tests/generator_tests/machines/invalid_network.txt".to_string();
    generate_sim(file_path).await;
}

#[tokio::test]
#[should_panic(expected = "Invalid Network ID found. Got 89 expected \"1 , 5\"")]
async fn generator_machine_invalid_network_id() {
    catch_panic();
    let file_path: String = "./tests/generator_tests/machines/invalid_network_id.txt".to_string();
    generate_sim(file_path).await;
}

#[tokio::test]
#[should_panic(expected = "Invalid application in machine. Got application bad_application")]
async fn generator_machine_invalid_application_name() {
    catch_panic();
    let file_path: String =
        "./tests/generator_tests/machines/invalid_application_name.txt".to_string();
    generate_sim(file_path).await;
}

#[tokio::test]
#[should_panic(expected = "Invalid name for 'to' in send_message, found: pizza")]
async fn generator_machine_invalid_machine_name_in_application() {
    catch_panic();
    let file_path: String = "./tests/generator_tests/machines/invalid_machine_name.txt".to_string();
    generate_sim(file_path).await;
}

#[tokio::test]
#[should_panic(expected = "Capture application doesn't contain message_count.")]
async fn generator_machine_missing_message_count_in_capture() {
    catch_panic();
    let file_path: String =
        "./tests/generator_tests/machines/missing_message_count_in_capture.txt".to_string();
    generate_sim(file_path).await;
}

#[tokio::test]
#[should_panic(
    expected = "Invalid IP found in capture application. IP does not exist in ip table. Found: [111, 11, 11, 11]"
)]
async fn generator_machine_invalid_ip_in_capture() {
    catch_panic();
    let file_path: String =
        "./tests/generator_tests/machines/invalid_ip_in_capture.txt".to_string();
    generate_sim(file_path).await;
}

#[tokio::test]
#[should_panic(expected = "Machine application does not contain a name")]
async fn generator_machine_missing_application_name() {
    catch_panic();
    let file_path: String =
        "./tests/generator_tests/machines/missing_application_name.txt".to_string();
    generate_sim(file_path).await;
}
