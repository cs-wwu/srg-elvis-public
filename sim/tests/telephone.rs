use elvis::{
    applications::{Capture, Forward, SendMessage},
    core::{Internet, Message, SharedProtocol},
    protocols::{ipv4::Ipv4, udp::Udp},
};

#[tokio::test(flavor = "multi_thread", worker_threads = 10)]
pub async fn telephone() {
    console_subscriber::init();
    let mut internet = Internet::new();
    let network = internet.network(1500);

    internet.machine(
        [
            Udp::new_shared() as SharedProtocol,
            Ipv4::new_shared(),
            SendMessage::new_shared(
                "Hello!",
                0u32.to_be_bytes().into(),
                1u32.to_be_bytes().into(),
                0xbeef,
                0xbeef,
            ),
        ],
        [network],
    );

    let end = 2;
    for i in 1u32..end {
        internet.machine(
            [
                Udp::new_shared() as SharedProtocol,
                Ipv4::new_shared(),
                Forward::new_shared(
                    i.to_be_bytes().into(),
                    (i + 1).to_be_bytes().into(),
                    0xbeef,
                    0xbeef,
                ),
            ],
            [network],
        );
    }

    let capture = Capture::new_shared(end.to_be_bytes().into(), 0xbeef);
    internet.machine(
        [
            Udp::new_shared() as SharedProtocol,
            Ipv4::new_shared(),
            capture.clone(),
        ],
        [network],
    );

    internet.run().await;
    assert_eq!(
        capture.lock().unwrap().application().message().unwrap(),
        Message::new("Hello!")
    );
}
