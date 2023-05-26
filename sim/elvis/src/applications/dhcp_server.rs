use elvis_core::{
    message::Message,
    protocol::Context,
    protocols::{
        dhcp_parsing::{DhcpMessage, MessageType},
        ipv4::Ipv4Address,
        sockets::{
            socket::{ProtocolFamily, Socket, SocketAddress, SocketType},
            Sockets,
        },
        user_process::{Application, ApplicationError, UserProcess},
    },
    Id, ProtocolMap, Shutdown,
};

use std::sync::{Arc, RwLock};
use tokio::sync::Barrier;

// Port number & broadcast frequency used by DHCP servers
pub const PORT_NUM: u16 = 67;
pub const BROADCAST: Ipv4Address = Ipv4Address::new([255, 255, 255, 255]);

/// A struct describing an implementation of a DHCP server
pub struct DhcpServer {
    // Sockets API
    sockets: Arc<Sockets>,
}

impl DhcpServer {
    pub fn new(sockets: Arc<Sockets>) -> Self {
        Self { sockets }
    }

    pub fn shared(self) -> Arc<UserProcess<Self>> {
        UserProcess::new(self).shared()
    }
}

/// Generate the next ip from curr_ip
pub fn gen_ipv4(
    curr_ip: Arc<RwLock<[u8; 4]>>,
    avail_ips: Arc<RwLock<Vec<Ipv4Address>>>,
) -> Ipv4Address {
    // Read from the lock and check if any IPs have been returned to the server
    if avail_ips.read().unwrap().is_empty() {
        let index = 3;
        get_next_ip(index, curr_ip.clone());
    } else {
        let client_addr = avail_ips.write().unwrap().remove(0);
        return client_addr;
    }
    Ipv4Address::new(*curr_ip.read().unwrap())
}

fn get_next_ip(index: usize, curr_ip: Arc<RwLock<[u8; 4]>>) {
    let c = *curr_ip.read().unwrap();
    if c == [255, 255, 255, 254] {
        return;
    }
    if index == 0 {
        if c[index] == 255 {}
    } else if c[index] == 255 {
        //if index is at max value
        curr_ip.write().unwrap()[index] = 0;
        get_next_ip(index - 1, curr_ip);
    } else if c[3] == 254 {
        println!("Break");
        curr_ip.write().unwrap()[index] = c[index] + 1;
    } else {
        curr_ip.write().unwrap()[index] = c[index] + 1;
    }
}

/// Perform the dynamic IP allocation process
/// As described in the DHCP RFC
async fn communicate_with_client(
    socket: Arc<Socket>,
    curr_ip: Arc<RwLock<[u8; 4]>>,
    avail_ips: Arc<RwLock<Vec<Ipv4Address>>>,
) {
    loop {
        let client_msg = socket.clone().recv_msg().await.unwrap();
        let parsed_client_msg = DhcpMessage::from_bytes(client_msg.iter()).unwrap();
        // match based on message type and respond accordingly
        match parsed_client_msg.msg_type {
            MessageType::Discover => {
                let new_addr = gen_ipv4(curr_ip, avail_ips);
                let mut resp = DhcpMessage::default();
                resp.your_ip = new_addr;
                println!("Server generated IP: {:?}", resp.your_ip);
                resp.op = 2;
                resp.your_ip = new_addr;
                resp.msg_type = MessageType::Offer;
                let resp_msg = DhcpMessage::to_message(resp).unwrap();
                let resp_msg = resp_msg.to_vec();
                socket.clone().send(resp_msg).unwrap();
                // this break to be removed when functionality is improved
                break;
            }
            MessageType::Request => {
                let new_addr = parsed_client_msg.your_ip;
                let mut resp = DhcpMessage::default();
                resp.op = 2;
                resp.your_ip = new_addr;
                resp.msg_type = MessageType::Offer;
                let resp_msg = DhcpMessage::to_message(resp).unwrap();
                let resp_msg = resp_msg.to_string();
                socket.clone().send(resp_msg).unwrap();
                break;
            }
            //invalid message type, send error
            _ => println!("Invalid message type!"),
        }
    }
}

impl Default for DhcpServer {
    fn default() -> Self {
        Self {
            sockets: Sockets::new(Some(BROADCAST)).shared(),
        }
    }
}

// We should move this into application in the 'elvis' branch eventually
impl Application for DhcpServer {
    const ID: Id = Id::from_string("DHCP Server");

    /// Initialize the server and listen/respond to client requests
    fn start(
        &self,
        _shutdown: Shutdown,
        initialized: Arc<Barrier>,
        protocols: ProtocolMap,
    ) -> Result<(), ApplicationError> {
        // take ownership of struct fields
        let sockets = self.sockets.clone();

        let current_ip = Arc::new(RwLock::new([0, 0, 0, 0]));
        let avail_ips = Arc::new(RwLock::new(Vec::<Ipv4Address>::new()));

        tokio::spawn(async move {
            // Create a new IPv4 Datagram Socket
            let listen_socket = sockets
                .clone()
                .new_socket(ProtocolFamily::INET, SocketType::Datagram, protocols)
                .await
                .unwrap();
            // Bind the socket for listening
            let local_sock_addr = SocketAddress::new_v4(BROADCAST, PORT_NUM);
            listen_socket.clone().bind(local_sock_addr).unwrap();

            // Listen for incoming connections, with an unlimited backlog
            listen_socket.clone().listen(0).unwrap();
            println!("SERVER: Listening for incoming connections");

            // Wait for OK from barrier
            initialized.wait().await;

            let mut tasks = Vec::new();

            // Accept new tasks until shutdown
            // With each task getting its own tokio thread
            loop {
                // Accept an incoming connection
                let socket = match listen_socket.clone().accept().await {
                    Ok(accepted) => accepted,
                    Err(_) => break,
                };
                println!("SERVER: Connection accepted");

                // Spawn a new tokio task for handling communication
                // with the new client
                let a = avail_ips.clone();
                let c = current_ip.clone();
                tasks.push(tokio::spawn(async move {
                    communicate_with_client(socket, c, a).await;
                }));
            }
        });
        Ok(())
    }

    fn receive(&self, _message: Message, _context: Context) -> Result<(), ApplicationError> {
        Ok(())
    }
}
