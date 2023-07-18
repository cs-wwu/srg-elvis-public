use elvis_core::{
    machine::ProtocolMap,
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{Endpoint, TcpListener, TcpStream},
    Control, Protocol, Session, Shutdown,
};
use std::sync::Arc;
use tokio::sync::Barrier;

pub struct TcpListenerServer {
    _client_address: Endpoint, // I don't know why, but the simulation breaks when this is removed
    server_address: Endpoint,
}

impl TcpListenerServer {
    pub fn new(client_address: Endpoint, server_address: Endpoint) -> Self {
        Self {
            _client_address: client_address,
            server_address,
        }
    }
}

#[async_trait::async_trait]
impl Protocol for TcpListenerServer {
    async fn start(
        &self,
        _shutdown: Shutdown,
        _initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        drop(_shutdown);

        // Create a new TcpListener bound to the server address
        let listener: TcpListener = TcpListener::bind(self.server_address, protocols)
            .await
            .unwrap();

        // Accept an incoming connection to create new TcpStream
        let mut stream: TcpStream = TcpListener::accept(&listener).await.unwrap();

        // Read up to 4 bytes from the client
        let max_bytes: usize = 4;
        let mut msg: Vec<u8> = stream.read(max_bytes).await.unwrap();

        // Add 1 to each number in the vec
        for n in &mut msg {
            *n += 1;
        }

        // Send the modified message back to the client
        stream.write(msg).await.unwrap();

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
