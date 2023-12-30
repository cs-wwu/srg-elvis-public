use elvis_core::{
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{Endpoint, TcpStream},
    Control, Machine, Protocol, Session, Shutdown,
};
use std::sync::Arc;
use tokio::sync::Barrier;

/// Client designed to test TcpListener and TcpStream
pub struct TcpStreamClient {
    _client_address: Endpoint, // I don't know why, but the simulation breaks when this is removed
    server_address: Endpoint,
}

impl TcpStreamClient {
    pub fn new(client_address: Endpoint, server_address: Endpoint) -> Self {
        Self {
            _client_address: client_address,
            server_address,
        }
    }
}

impl Protocol for TcpStreamClient {
    async fn start(
        &self,
        shutdown: Shutdown,
        initialized: Arc<Barrier>,
        machine: Arc<Machine>,
    ) -> Result<(), StartError> {
        initialized.wait().await;
        // Create a new TcpStream connected to the server address
        let mut stream: TcpStream = TcpStream::connect(self.server_address, machine)
            .await
            .unwrap();

        // TESTING TcpStream::read_exact()
        // Send bytes to the server
        let mut msg1: Vec<u8> = vec![
            rand::random::<u8>() / 2,
            rand::random::<u8>() / 2,
            rand::random::<u8>() / 2,
        ];
        stream.write(msg1.clone()).await.unwrap();

        // Recieve bytes from the server using read_exact
        let max_bytes: usize = 4;
        let received_msg1: Vec<u8> = stream.read_exact(max_bytes).await.unwrap();

        // Add 1 to each number in the vec
        for n in &mut msg1 {
            *n += 1;
        }

        // The message sent and recieved should be identical
        assert_eq!(msg1, received_msg1);

        // TESTING TcpStream::read()
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

        // Shut down the simulation
        shutdown.shut_down();

        Ok(())
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
