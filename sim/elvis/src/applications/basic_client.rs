use elvis_core::{
    machine::ProtocolMap,
    message::Message,
    protocol::{DemuxError, StartError},
    protocols::{dhcp_client::DhcpClient, ipv4::Ipv4Address, Endpoint, Endpoints, Tcp, Udp},
    Control, Protocol, Session, Shutdown, Transport,
};
use std::{sync::Arc, time::Duration};
use tokio::{sync::Barrier, time::sleep};

pub struct BasicClient {
    /// Numerical ID
    client_id: u16,
    /// The endpoint to send to
    endpoint: Endpoint,
    /// the application's local address
    local_ip: Ipv4Address,
    /// Whether to use UDP or TCP
    transport: Transport,
    /// Whether to output text or not
    output: bool,
    /// The (optional) delay between clients
    delay_ms: u16,
}

impl BasicClient {
    pub fn new(
        client_id: u16,
        endpoint: Endpoint,
        local_ip: Ipv4Address,
        transport: Transport,
        output: bool,
        delay_ms: u16,
    ) -> Self {
        Self {
            client_id,
            endpoint,
            local_ip,
            transport,
            output,
            delay_ms,
        }
    }
}

#[async_trait::async_trait]
impl Protocol for BasicClient {
    async fn start(
        &self,
        _shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        let endpoint = self.endpoint;
        let transport = self.transport;

        initialized.wait().await;

        if self.delay_ms > 0 {
            let duration: u64 = (self.delay_ms * self.client_id).into();
            sleep(Duration::from_millis(duration)).await;
        }

        let local_address = match protocols.protocol::<DhcpClient>() {
            Some(dhcp) => dhcp.ip_address().await,
            None => self.local_ip,
        };

        let endpoints = Endpoints {
            local: Endpoint {
                address: local_address,
                port: 0,
            },
            remote: endpoint,
        };

        let session = match transport {
            Transport::Tcp => protocols
                .protocol::<Tcp>()
                .unwrap()
                .open(self.id(), endpoints, protocols.clone())
                .await
                .unwrap(),
            Transport::Udp => protocols
                .protocol::<Udp>()
                .unwrap()
                .open_and_listen(self.id(), endpoints, protocols.clone())
                .await
                .unwrap(),
        };

        let req = format!("({}) Ground Control to Major Tom", self.client_id);
        if self.output {
            println!("CLIENT ({}): Sending Request: {:?}", self.client_id, req)
        }
        session.send(Message::new(req), protocols.clone()).unwrap();
        Ok(())
    }

    fn demux(
        &self,
        message: Message,
        caller: Arc<dyn Session>,
        _control: Control,
        protocols: ProtocolMap,
    ) -> Result<(), DemuxError> {
        if self.output {
            println!(
                "CLIENT ({}): Response Received: {:?}",
                self.client_id,
                String::from_utf8(message.to_vec()).unwrap()
            )
        }
        let ack = format!("({}) Acknowledged", self.client_id);
        if self.output {
            println!(
                "CLIENT ({}): Sending Acknowledgement: {:?}",
                self.client_id, ack
            )
        }
        caller.send(Message::new(ack), protocols).unwrap();

        Ok(())
    }
}
