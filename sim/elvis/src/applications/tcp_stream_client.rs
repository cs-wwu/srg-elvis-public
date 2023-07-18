use elvis_core::{
    machine::ProtocolMap,
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{Endpoint, TcpStream},
    Control, Protocol, Session, Shutdown,
};
use std::sync::Arc;
use tokio::sync::Barrier;

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

#[async_trait::async_trait]
impl Protocol for TcpStreamClient {
    async fn start(
        &self,
        shutdown: Shutdown,
        _initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        // Create a new TcpStream connected to the server address
        let mut stream: TcpStream = TcpStream::connect(self.server_address, protocols)
            .await
            .unwrap();

        // Send bytes to the server
        let mut msg: Vec<u8> = vec![
            rand::random::<u8>() / 2,
            rand::random::<u8>() / 2,
            rand::random::<u8>() / 2,
        ];
        stream.write(msg.clone()).await.unwrap();

        // Recieve bytes from the server
        let max_bytes: usize = 4;
        let received_msg: Vec<u8> = stream.read(max_bytes).await.unwrap();

        // Add 1 to each number in the vec
        for n in &mut msg {
            *n += 1;
        }

        // The message sent and recieved should be identical
        assert_eq!(msg, received_msg);

        // Shut down the simulation
        shutdown.shut_down();

        Ok(())
    }

    fn demux(
        &self,
        _message: Message,
        _caller: Arc<dyn Session>,
        _control: Control,
        _protocols: ProtocolMap,
    ) -> Result<(), DemuxError> {
        Ok(())
    }
}
