use elvis_core::{
    machine::ProtocolMap,
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{Endpoint, TcpListener, TcpStream},
    Control, Protocol, Session, Shutdown,
};
use std::sync::Arc;
use tokio::sync::Barrier;

/// Very simple web server designed to test ELVIS's speed. Connects to a BareBonesClient and
/// repeatedly reads a vec of u8's from the client, adds 1 to each value, and writes the data back
/// to the client
pub struct BareBonesServer {
    server_address: Endpoint,
}

impl BareBonesServer {
    pub fn new(server_address: Endpoint) -> Self {
        Self { server_address }
    }

    async fn handle_connection(mut stream: TcpStream) {
        loop {
            let mut msg2: Vec<u8> = stream.read().await.unwrap();

            // Add 1 to each number in the vec
            for n in &mut msg2 {
                *n += 1;
            }

            // Send the modified message back to the client
            stream.write(msg2).await.unwrap();
        }
    }
}

#[async_trait::async_trait]
impl Protocol for BareBonesServer {
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

        loop {
            // Accept an incoming connection to create new TcpStream
            let stream: TcpStream = TcpListener::accept(&listener).await.unwrap();

            tokio::spawn(async move {
                BareBonesServer::handle_connection(stream).await;
            });
        }
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
