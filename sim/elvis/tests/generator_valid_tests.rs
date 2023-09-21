//! Tests on the NDL generator, for valid tests
use elvis::ndl::generate_and_run_sim;

#[tokio::test]
#[ntest::timeout(200)]
async fn basic_message_valid_test() {
    let file_path: String =
        "./tests/generator_tests/valid/basic_message_valid_test.txt".to_string();
    generate_and_run_sim(file_path).await;
}

#[tokio::test]
#[ntest::timeout(200)]
async fn basic_forward_valid_test() {
    let file_path: String =
        "./tests/generator_tests/valid/basic_forward_valid_test.txt".to_string();
    generate_and_run_sim(file_path).await;
}

#[tokio::test]
#[ntest::timeout(200)]
async fn basic_pingpong_valid_test() {
    let file_path: String =
        "./tests/generator_tests/valid/basic_pingpong_valid_test.txt".to_string();
    generate_and_run_sim(file_path).await;
}

#[tokio::test]
#[ntest::timeout(200)]
async fn basic_message_ip_valid_test() {
    let file_path: String =
        "./tests/generator_tests/valid/basic_message_ip_valid_test.txt".to_string();
    generate_and_run_sim(file_path).await;
}

#[tokio::test]
#[ntest::timeout(200)]
async fn basic_forward_ip_valid_test() {
    let file_path: String =
        "./tests/generator_tests/valid/basic_forward_ip_valid_test.txt".to_string();
    generate_and_run_sim(file_path).await;
}

#[tokio::test]
#[ntest::timeout(200)]
async fn basic_pingpong_ip_valid_test() {
    let file_path: String =
        "./tests/generator_tests/valid/basic_pingpong_ip_valid_test.txt".to_string();
    generate_and_run_sim(file_path).await;
}
