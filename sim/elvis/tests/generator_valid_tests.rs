//! Tests on the NDL generator, for valid tests
use elvis::ndl::generate_and_run_sim;

#[tokio::test]
#[ntest::timeout(7000)]
async fn basic_message_valid_test() {
    let file_path: String =
        "./tests/generator_tests/valid/basic_valid/basic_message_valid_test.txt".to_string();
    generate_and_run_sim(file_path).await;
}

#[tokio::test]
#[ntest::timeout(7000)]
async fn basic_forward_valid_test() {
    let file_path: String =
        "./tests/generator_tests/valid/basic_valid/basic_forward_valid_test.txt".to_string();
    generate_and_run_sim(file_path).await;
}

#[tokio::test]
#[ntest::timeout(7000)]
async fn basic_pingpong_valid_test() {
    let file_path: String =
        "./tests/generator_tests/valid/basic_valid/basic_pingpong_valid_test.txt".to_string();
    generate_and_run_sim(file_path).await;
}

#[tokio::test]
#[ntest::timeout(7000)]
async fn basic_message_ip_valid_test() {
    let file_path: String =
        "./tests/generator_tests/valid/basic_valid/basic_message_ip_valid_test.txt".to_string();
    generate_and_run_sim(file_path).await;
}

#[tokio::test]
#[ntest::timeout(7000)]
async fn basic_forward_ip_valid_test() {
    let file_path: String =
        "./tests/generator_tests/valid/basic_valid/basic_forward_ip_valid_test.txt".to_string();
    generate_and_run_sim(file_path).await;
}

#[tokio::test]
#[ntest::timeout(7000)]
async fn basic_pingpong_ip_valid_test() {
    let file_path: String =
        "./tests/generator_tests/valid/basic_valid/basic_pingpong_ip_valid_test.txt".to_string();
    generate_and_run_sim(file_path).await;
}
//----------------------------------------------------------------
#[tokio::test]
async fn multi_arp_multi_capture() {
    let file_path: String =
        "./tests/generator_tests/valid/arp_router_valid/multi_arp_multi_capture.txt".to_string();
    generate_and_run_sim(file_path).await;
}

#[tokio::test]
async fn multi_arp_pingpong() {
    let file_path: String =
        "./tests/generator_tests/valid/arp_router_valid/multi_arp_pingpong.txt".to_string();
    generate_and_run_sim(file_path).await;
}

#[tokio::test]
async fn multi_arp_single_capture() {
    let file_path: String =
        "./tests/generator_tests/valid/arp_router_valid/multi_arp_single_capture.txt".to_string();
    generate_and_run_sim(file_path).await;
}

#[tokio::test]
async fn single_arp_multi_capture() {
    let file_path: String =
        "./tests/generator_tests/valid/arp_router_valid/single_arp_multi_capture.txt".to_string();
    generate_and_run_sim(file_path).await;
}

#[tokio::test]
async fn single_arp_pingpong() {
    let file_path: String =
        "./tests/generator_tests/valid/arp_router_valid/single_arp_pingpong.txt".to_string();
    generate_and_run_sim(file_path).await;
}

#[tokio::test]
async fn single_arp_single_capture() {
    let file_path: String =
        "./tests/generator_tests/valid/arp_router_valid/single_arp_single_capture.txt".to_string();
    generate_and_run_sim(file_path).await;
}

#[tokio::test]
async fn multi_rip_multi_capture() {
    let file_path: String =
        "./tests/generator_tests/valid/rip_router_valid/multi_rip_multi_capture.txt".to_string();
    generate_and_run_sim(file_path).await;
}

#[tokio::test]
async fn multi_rip_single_capture() {
    let file_path: String =
        "./tests/generator_tests/valid/rip_router_valid/multi_rip_single_capture.txt".to_string();
    generate_and_run_sim(file_path).await;
}

#[tokio::test]
async fn single_rip_single_capture() {
    let file_path: String =
        "./tests/generator_tests/valid/rip_router_valid/single_rip_single_capture.txt".to_string();
    generate_and_run_sim(file_path).await;
}
