use std::sync::Arc;

use super::{
    socket_api::socket::{ProtocolFamily, Socket, SocketError, SocketType},
    tcp_stream::TcpStream,
    Endpoint,
};

use crate::{machine::ProtocolMap, protocols::SocketAPI};

pub struct TcpListener {
    local_socket: Arc<Socket>,
}

impl TcpListener {
    // Creates a new TcpListener bound to the given socket address
    pub async fn bind(
        socket_address: Endpoint,
        protocols: ProtocolMap,
    ) -> Result<Self, SocketError> {
        let sockets_api = protocols.protocol::<SocketAPI>().unwrap();
        let socket = SocketAPI::new_socket(
            &sockets_api,
            ProtocolFamily::INET,
            SocketType::Stream,
            protocols.clone(),
        )
        .await?;
        socket.bind(socket_address)?;
        socket.listen(1000)?;

        Ok(Self {
            local_socket: socket,
        })
    }

    // Creates a new TcpStream bound to the local socket
    pub async fn accept(&self) -> Result<TcpStream, SocketError> {
        let remote_socket = self.local_socket.accept().await?;
        let stream = TcpStream {
            local_socket: remote_socket,
        };
        Ok(stream)
    }
}
