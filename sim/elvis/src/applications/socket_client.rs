use elvis_core::{
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{
        ipv4::Ipv4Address,
        socket_api::socket::{ProtocolFamily, SocketType},
        Endpoint, SocketAPI,
    },
    Control, Machine, Protocol, Session, Shutdown,
};
use std::{any::TypeId, sync::Arc, time::Duration};
use tokio::{sync::Barrier, time::sleep};

pub struct SocketClient {
    /// Numerical ID
    client_id: u16,
    /// The IP address to send to
    remote_ip: Ipv4Address,
    /// The port to send to
    remote_port: u16,
    /// Whether to use UDP or TCP
    transport: SocketType,
    /// Whether to output text or not
    output: bool,
    /// The (optional) delay between clients
    delay_ms: u16,
}

impl SocketClient {
    pub fn new(
        client_id: u16,
        remote_ip: Ipv4Address,
        remote_port: u16,
        transport: SocketType,
        output: bool,
        delay_ms: u16,
    ) -> Self {
        Self {
            client_id,
            remote_ip,
            remote_port,
            transport,
            output,
            delay_ms,
        }
    }
}

#[async_trait::async_trait]
impl Protocol for SocketClient {
    async fn start(
        &self,
        _shutdown: Shutdown,
        initialized: Arc<Barrier>,
        machine: Arc<Machine>,
    ) -> Result<(), StartError> {
        drop(_shutdown);
        // Take ownership of struct fields so they can be accessed within the
        // tokio thread
        let sockets = machine
            .protocol::<SocketAPI>()
            .ok_or(StartError::MissingProtocol(TypeId::of::<SocketAPI>()))?;

        let mut socket = sockets
            .new_socket(ProtocolFamily::INET, self.transport, machine)
            .await
            .unwrap();

        // Wait on initialization before sending any message across the network
        initialized.wait().await;

        if self.delay_ms > 0 {
            let duration: u64 = (self.delay_ms * self.client_id).into();
            sleep(Duration::from_millis(duration)).await;
        }

        // "Connect" the socket to a remote address
        let remote_sock_addr = Endpoint::new(self.remote_ip, self.remote_port);
        socket.connect(remote_sock_addr).await.unwrap();

        if self.output {
            println!("CLIENT ({}): Connected", self.client_id);
        }

        // Error checking, these calls *should* return errors.
        if socket.listen(10).is_ok() {
            return Err(StartError::Other);
        }
        if socket.connect(remote_sock_addr).await.is_ok() {
            return Err(StartError::Other);
        }

        // Send a message
        let req = format!("({}) Ground Control to Major Tom", self.client_id);
        if self.output {
            println!("CLIENT ({}): Sending Request: {:?}", self.client_id, req);
        }
        socket.send(req).unwrap();

        // Receive a message
        match socket.recv_msg().await {
            Ok(resp) => {
                if self.output {
                    println!(
                        "CLIENT ({}): Response Received: {:?}",
                        self.client_id,
                        String::from_utf8(resp.to_vec()).unwrap()
                    )
                };
            }
            Err(e) => {
                println!("Client ({:?}) Error: {:?}", self.client_id, e)
            }
        }

        // Send an acknowledgement message
        let ack = format!("({}) Acknowledged", self.client_id);
        if self.output {
            println!(
                "CLIENT ({}): Sending Acknowledgement: {:?}",
                self.client_id, ack
            );
        }
        socket.send(ack).unwrap();
        socket.close();
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
