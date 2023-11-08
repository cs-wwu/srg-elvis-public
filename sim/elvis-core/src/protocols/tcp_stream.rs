use super::{
    socket_api::socket::{ProtocolFamily, Socket, SocketError, SocketType},
    Endpoint,
};
use crate::{machine::ProtocolMap, message::Chunk, protocols::SocketAPI};

pub struct TcpStream {
    pub local_socket: Socket,
}

impl TcpStream {
    /// Creates a new TcpStream connected to the given remote socket address
    pub async fn connect(
        remote_address: Endpoint,
        protocols: ProtocolMap,
    ) -> Result<Self, SocketError> {
        let sockets_api = protocols.protocol::<SocketAPI>().unwrap();
        let mut socket = sockets_api
            .new_socket(ProtocolFamily::INET, SocketType::Stream, protocols)
            .await?;
        socket.connect(remote_address).await?;

        Ok(Self {
            local_socket: socket,
        })
    }

    /// Read all bytes from the queue
    pub async fn read(&mut self) -> Result<Vec<u8>, SocketError> {
        let msg = self.local_socket.recv_msg().await?;

        Ok(msg.to_vec())
    }

    /// Writes data to the remote socket bound to the local socket
    pub async fn write(
        &mut self,
        message: impl Into<Chunk> + std::marker::Send + 'static,
    ) -> Result<(), SocketError> {
        self.local_socket.send(message)
    }

    /// Receives at most 'bytes' data from the remote socket bound to the local socket
    pub async fn read_exact(&mut self, bytes: usize) -> Result<Vec<u8>, SocketError> {
        self.local_socket.recv(bytes).await
    }
}
