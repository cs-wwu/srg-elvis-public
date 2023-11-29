use std::sync::Arc;

use super::{
    socket_api::socket::{ProtocolFamily, Socket, SocketError, SocketType},
    tcp_stream::TcpStream,
    Endpoint,
};

use crate::{protocols::SocketAPI, Machine};

pub struct TcpListener {
    local_socket: Socket,
}

impl TcpListener {
    /// Creates a new TcpListener bound to the given socket address
    pub async fn bind(
        socket_address: Endpoint,
        machine: Arc<Machine>,
    ) -> Result<Self, SocketError> {
        let sockets_api = machine.protocol::<SocketAPI>().unwrap();
        let mut socket = sockets_api
            .new_socket(ProtocolFamily::INET, SocketType::Stream, machine.clone())
            .await?;
        socket.bind(socket_address)?;
        socket.listen(5000)?;

        Ok(Self {
            local_socket: socket,
        })
    }

    /// Creates a new TcpStream bound to the local socket
    pub async fn accept(&mut self) -> Result<TcpStream, SocketError> {
        let remote_socket = self.local_socket.accept().await?;
        let stream = TcpStream {
            local_socket: remote_socket,
        };
        Ok(stream)
    }
}
