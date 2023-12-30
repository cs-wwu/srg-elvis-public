use elvis_core::{
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{socket_api::socket::SocketError, Endpoint, TcpListener, TcpStream},
    Control, Machine, Protocol, Session, Shutdown,
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
            let mut msg2: Vec<u8> = match stream.read().await {
                Ok(v) => v,
                Err(SocketError::Shutdown) => {
                    break;
                }
                Err(e) => panic!("{:?}", e),
            };

            // Add 1 to each number in the vec
            for n in &mut msg2 {
                *n += 1;
            }

            // Send the modified message back to the client
            stream.write(msg2).await.unwrap();
        }
    }
}

impl Protocol for BareBonesServer {
    async fn start(
        &self,
        _shutdown: Shutdown,
        initialized: Arc<Barrier>,
        machine: Arc<Machine>,
    ) -> Result<(), StartError> {
        drop(_shutdown);

        // Create a new TcpListener bound to the server address
        let mut listener: TcpListener = TcpListener::bind(self.server_address, machine)
            .await
            .unwrap();

        initialized.wait().await;

        loop {
            // Accept an incoming connection to create new TcpStream
            let stream: TcpStream = match TcpListener::accept(&mut listener).await {
                Ok(v) => v,
                Err(_) => {
                    break (Ok(()));
                }
            };

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
        _machine: Arc<Machine>,
    ) -> Result<(), DemuxError> {
        Ok(())
    }
}
