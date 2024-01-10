//! Tests on generator machine functions
use elvis::ndl::generate_and_run_sim;
use elvis_core::ExitStatus;
use tokio::time::Duration;

/// Catches an intended panic
/// used for when a test should fail as intended
fn catch_panic() {
    std::panic::set_hook(Box::new(|_info| {
        // do nothing
    }));
}

#[tokio::test(flavor = "multi_thread")]
#[should_panic(expected = "Invalid Protocol found in machine. Found: BadProtocol")]
async fn generator_machine_invalid_protocol() {
    catch_panic();
    let file_path: String =
        "./tests/generator_tests/machines/general/invalid_protocol.txt".to_string();
    generate_and_run_sim(file_path, None).await;
}

#[tokio::test(flavor = "multi_thread")]
#[should_panic(expected = "No ID found in network being added to machine.")]
async fn generator_machine_invalid_network() {
    catch_panic();
    let file_path: String =
        "./tests/generator_tests/machines/general/invalid_network.txt".to_string();
    generate_and_run_sim(file_path, None).await;
}

#[tokio::test(flavor = "multi_thread")]
#[should_panic(expected = "Invalid Network ID found. Got 89 expected \"1 , 5\"")]
async fn generator_machine_invalid_network_id() {
    catch_panic();
    let file_path: String =
        "./tests/generator_tests/machines/general/invalid_network_id.txt".to_string();
    generate_and_run_sim(file_path, None).await;
}

#[tokio::test(flavor = "multi_thread")]
#[should_panic(expected = "Invalid application in machine. Got application bad_application")]
async fn generator_machine_invalid_application_name() {
    catch_panic();
    let file_path: String =
        "./tests/generator_tests/machines/general/invalid_application_name.txt".to_string();
    generate_and_run_sim(file_path, None).await;
}

#[tokio::test(flavor = "multi_thread")]
#[should_panic(expected = "Invalid name for 'to' in send_message, found: pizza")]
async fn generator_machine_invalid_machine_name_in_application() {
    catch_panic();
    let file_path: String =
        "./tests/generator_tests/machines/general/invalid_machine_name.txt".to_string();
    generate_and_run_sim(file_path, None).await;
}

#[tokio::test(flavor = "multi_thread")]
#[should_panic(expected = "Machine application does not contain a name")]
async fn generator_machine_missing_application_name() {
    catch_panic();
    let file_path: String =
        "./tests/generator_tests/machines/general/missing_application_name.txt".to_string();
    generate_and_run_sim(file_path, None).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn generator_machine_incomplete_factory() {
    let file_path: String =
        "./tests/generator_tests/machines/capture/incomplete_factory.txt".to_string();

    let timeout = Some(Duration::from_secs(5));
    let result = generate_and_run_sim(file_path, timeout).await;

    match result {
        Some(status) => {
            assert_eq!(
                status,
                ExitStatus::TimedOut,
                "Test should have exited with TimedOut, exited with {:?} instead",
                status
            )
        }
        None => panic!("Unable to parse file"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn generator_machine_incomplete_message_received() {
    let file_path: String =
        "./tests/generator_tests/machines/capture/incomplete_message_received.txt".to_string();
    let timeout = Some(Duration::from_secs(5));
    let result = generate_and_run_sim(file_path, timeout).await;

    match result {
        Some(status) => {
            assert_eq!(
                status,
                ExitStatus::TimedOut,
                "Test should have exited with TimedOut, exited with {:?} instead",
                status
            )
        }
        None => panic!("Unable to parse file"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn generator_machine_insufficient_messages_received() {
    let file_path: String =
        "./tests/generator_tests/machines/capture/insufficient_messages_received.txt".to_string();
    let timeout = Some(Duration::from_secs(5));
    let result = generate_and_run_sim(file_path, timeout).await;

    match result {
        Some(status) => {
            assert_eq!(
                status,
                ExitStatus::TimedOut,
                "Test should have exited with TimedOut, exited with {:?} instead",
                status
            )
        }
        None => panic!("Unable to parse file"),
    }
}

#[tokio::test(flavor = "multi_thread")]
#[should_panic(expected = "Invalid parameter for argument type, got bad_type")]
async fn generator_machine_invalid_capture_type() {
    catch_panic();
    let file_path: String =
        "./tests/generator_tests/machines/capture/invalid_capture_type.txt".to_string();
    generate_and_run_sim(file_path, None).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn generator_machine_invalid_message_received() {
    let file_path: String =
        "./tests/generator_tests/machines/capture/invalid_message_received.txt".to_string();

    let timeout = Some(Duration::from_secs(5));
    let result = generate_and_run_sim(file_path, timeout).await;

    match result {
        Some(status) => {
            assert_eq!(
                status,
                ExitStatus::TimedOut,
                "Test should have exited with TimedOut, exited with {:?} instead",
                status
            )
        }
        None => panic!("Unable to parse file"),
    }
}
