//! Tests on the NDL generator, for valid tests
use elvis::ndl::generate_and_run_sim;

#[tokio::test(flavor = "multi_thread")]
#[ntest::timeout(1000)]
async fn basic_message_valid_test() {
    let file_path: String = "./tests/generator_tests/valid/basic/message_valid.txt".to_string();
    generate_and_run_sim(file_path, None).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ntest::timeout(1000)]
async fn basic_forward_valid_test() {
    let file_path: String = "./tests/generator_tests/valid/basic/forward_valid.txt".to_string();
    generate_and_run_sim(file_path, None).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ntest::timeout(1000)]
async fn basic_pingpong_valid_test() {
    let file_path: String = "./tests/generator_tests/valid/basic/pingpong_valid.txt".to_string();
    generate_and_run_sim(file_path, None).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ntest::timeout(1000)]
async fn basic_message_ip_valid_test() {
    let file_path: String = "./tests/generator_tests/valid/basic/message_ip_valid.txt".to_string();
    generate_and_run_sim(file_path, None).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ntest::timeout(1000)]
async fn basic_forward_ip_valid_test() {
    let file_path: String = "./tests/generator_tests/valid/basic/forward_ip_valid.txt".to_string();
    generate_and_run_sim(file_path, None).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ntest::timeout(1000)]
async fn basic_pingpong_ip_valid_test() {
    let file_path: String = "./tests/generator_tests/valid/basic/pingpong_ip_valid.txt".to_string();
    generate_and_run_sim(file_path, None).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ntest::timeout(1000)]
async fn single_capture_test() {
    let file_path: String = "./tests/generator_tests/valid/capture/single_capture.txt".to_string();
    generate_and_run_sim(file_path, None).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ntest::timeout(1000)]
async fn single_message_test() {
    let file_path: String = "./tests/generator_tests/valid/capture/single_message.txt".to_string();
    generate_and_run_sim(file_path, None).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ntest::timeout(1000)]
async fn single_capture_multi_rec_test() {
    let file_path: String =
        "./tests/generator_tests/valid/capture/single_capture_multi_rec.txt".to_string();
    generate_and_run_sim(file_path, None).await;
}

#[tokio::test(flavor = "multi_thread")]
// #[ntest::timeout(1000)]
async fn factory_capture_test() {
    let file_path: String = "./tests/generator_tests/valid/capture/factory_capture.txt".to_string();
    generate_and_run_sim(file_path, None).await;
}
