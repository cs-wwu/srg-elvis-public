// use elvis_core::{
//     machine::ProtocolMap,
//     message::Message,
//     protocol::{DemuxError, StartError},
//     protocols::{
//         ipv4::Ipv4Address,
//         socket_api::socket::{ProtocolFamily, Socket, SocketType},
//         Endpoint, SocketAPI,
//     },
//     Control, Protocol, Session, Shutdown,
// };
// use std::{any::TypeId, sync::Arc};
// use tokio::sync::Barrier;

// pub struct DnsClient {
//     /// The Sockets API
//     sockets: Arc<Socket>,

//     /// Numerical ID
//     client_id: u16,

//     /// The text of the message to send
//     text: &'static str,

//     /// The IP address to send to
//     remote_ip: Ipv4Address,
    
//     /// The port to send to
//     remote_port: u16,
// }

// impl DnsClient {
//     pub fn new(
//         sockets: Arc<Socket>,
//         text: &'static str,
//         remote_ip: Ipv4Address,
//         remote_port: u16,
//     ) -> Self {
//         Self {
//             sockets,
//             text,
//             remote_ip,
//             remote_port,
//         }
//     }

//     pub fn shared(self) -> Arc<UserProcess<Self>> {
//         UserProcess::new(self).shared()
//     }
// }

// impl Protocol for DnsClient {
//     // const ID: Id = Id::from_string("Dns Client");

//     async fn start(
//         &self,
//         _shutdown: Sender<()>,
//         initialized: Arc<Barrier>,
//         protocols: ProtocolMap,
//     ) -> Result<(), ApplicationError> {
//         // Create a new IPv4 Datagram Socket
//         let socket = self
//             .sockets
//             .clone()
//             .new_socket(ProtocolFamily::INET, SocketType::SocketDatagram, protocols)
//             .unwrap();
//         let remote_ip = self.remote_ip;
//         let remote_port = self.remote_port;
//         let text = self.text;

//         tokio::spawn(async move {
//             // "Connect" the socket to a remote address
//             let remote_sock_addr = SocketAddress::new_v4(remote_ip, remote_port);
//             socket.clone().connect(remote_sock_addr).unwrap();


//             // Wait on initialization before sending any message across the network
//             initialized.wait().await;


//             // Send a connection request
//             println!("CLIENT: Sending connection request");
//             socket.clone().send("SYN").unwrap();

//             // Receive a connection response
//             let _ack = socket.clone().recv(32).await.unwrap();
//             println!("CLIENT: Connection response received");

//             // Send a message
//             println!("CLIENT: Sending Request: {:?}", text);
//             socket.clone().send(text).unwrap();

//             // Receive a message
//             let msg = socket.clone().recv(32).await.unwrap();
//             println!(
//                 "CLIENT: Response Received: {:?}",
//                 String::from_utf8(msg).unwrap()
//             );

//             // Send another message
//             println!("CLIENT: Sending Request: \"Shutdown\"");
//             socket.clone().send("Shutdown").unwrap();
//         });
//         Ok(())
//     }

//     fn receive(&self, _message: Message, _context: Context) -> Result<(), ApplicationError> {
//         Ok(())
//     }
// }