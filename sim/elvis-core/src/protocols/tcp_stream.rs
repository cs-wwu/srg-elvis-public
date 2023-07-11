use super::{
    socket_api::socket::{ProtocolFamily, Socket, SocketError, SocketType},
    Endpoint,
};
use std::sync::Arc;

use crate::{machine::ProtocolMap, message::Chunk, protocols::SocketAPI};

pub struct TcpStream {
    pub local_socket: Arc<Socket>,
}

impl TcpStream {
    // Creates a new TcpStream connected to the given remote socket address
    pub async fn connect(
        remote_address: Endpoint,
        protocols: ProtocolMap,
    ) -> Result<Self, SocketError> {
        let sockets_api = protocols.protocol::<SocketAPI>().unwrap();
        let socket = SocketAPI::new_socket(
            &sockets_api,
            ProtocolFamily::INET,
            SocketType::Datagram,
            protocols,
        )
        .await?;
        socket.connect(remote_address).await?;

        Ok(Self {
            local_socket: socket,
        })
    }

    // Receives at most 'bytes' data from the remote socket bound to the local socket
    pub async fn read(&mut self, bytes: usize) -> Result<Vec<u8>, SocketError> {
        self.local_socket.recv(bytes).await
    }

    // Writes data to the remote socket bound to the local socket
    pub async fn write(
        &mut self,
        message: impl Into<Chunk> + std::marker::Send + 'static,
    ) -> Result<(), SocketError> {
        self.local_socket.send(message)
    }
}
