use elvis_core::{
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{Endpoint, TcpStream},
    Control, Machine, Protocol, Session, Shutdown,
};
use std::sync::{Arc, RwLock};
use tokio::sync::Barrier;

/// Very simple web client designed to test ELVIS's speed. Connects to a BareBonesServer
/// and repeatedly sends a vec of 3 random u8's then reads from the server and checks that the
/// server sent back a vec of those same u8's with one added to each value.
pub struct BareBonesClient {
    server_address: Endpoint,
    pub num_pages_recvd: RwLock<u32>,
}

impl BareBonesClient {
    pub fn new(server_address: Endpoint) -> Self {
        Self {
            server_address,
            /// Tracks the number of web pages recieved by this client
            num_pages_recvd: RwLock::new(0),
        }
    }
}

#[async_trait::async_trait]
impl Protocol for BareBonesClient {
    async fn start(
        &self,
        _shutdown: Shutdown,
        _initialized: Arc<Barrier>,
        machine: Arc<Machine>,
    ) -> Result<(), StartError> {
        // Create a new TcpStream connected to the server address
        let mut stream: TcpStream = TcpStream::connect(self.server_address, machine)
            .await
            .unwrap();

        loop {
            // Send bytes to the server
            let mut msg2: Vec<u8> = vec![
                rand::random::<u8>() / 2,
                rand::random::<u8>() / 2,
                rand::random::<u8>() / 2,
            ];
            stream.write(msg2.clone()).await.unwrap();

            // Recieve bytes from the server using read_exact
            let max_bytes: usize = 4;
            let received_msg2: Vec<u8> = stream.read_exact(max_bytes).await.unwrap();

            // Add 1 to each number in the vec
            for n in &mut msg2 {
                *n += 1;
            }

            // The message sent and recieved should be identical
            assert_eq!(msg2, received_msg2);

            // Iterate num_pages_recvd
            *self.num_pages_recvd.write().unwrap() += 1;
        }
    }

    fn demux(
        &self,
        _message: Message,
        _caller: Arc<dyn Session>,
        _control: Control,
        _machine: Arc<Machine>,
    ) -> Result<(), DemuxError> {
        Ok(())
    }
}
