//! The terminal protocol is an application that lives above UDP.
//! The application communicates with an actual port on the real-world machine running ELVIS
//! and sends and receives messages over this port via TCP communication with a command terminal.

use elvis_core::protocols::{Endpoint, Endpoints, Udp};
use elvis_core::protocols::dhcp_client::DhcpClient;
use elvis_core::{*, protocols::ipv4::Ipv4Address};
use elvis_core::machine::*;
use elvis_core::session::Session;
use elvis_core::protocol::*;
use tokio::sync::Barrier;
use std::sync::{Arc, RwLock};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
};
// use std::any::*;

#[derive(Debug)]
pub struct Terminal {
    // local_ip: Ipv4Address,
    endpoint: Endpoint,
    /// The queue of messages received (qpush) by the application that can be
    /// returned (qpop) when a fetch request is made.
    msg_queue: RwLock<Vec<String>>,
    /// The real-world port to communicate with the physical machine's terminal over.
    port: String,
    transport: Transport,
}

impl Terminal {
    pub fn new(
        local: Endpoint,
        assign_port: String
    ) -> Terminal {
        Self {
            endpoint: local,
            msg_queue: RwLock::new(Vec::new()),
            port: assign_port,
            // local_ip: Ipv4Address::LOCALHOST,
            transport: Transport::Udp,
        }
    }

    pub fn parse(
        string: String,
    ) -> Result<TerminalCommand, TerminalError> {
        // Split command by spaces
        let args: Vec<&str> = string
            .split(" ")
            .collect();

        // Determine if command starts with "send" or "fetch"
        match args[0].trim().as_ref() {
            "send" => {
                println!("Send");
                // Check size of args == 3
                if args.len() != 3 {
                    Terminal::usage();
                    Err(TerminalError::TERROR)
                } else {
                    // Parse args[1] into Ipv4Address and port
                    let adr_and_port: Vec<&str> = args[1]
                        .split(":")
                        .collect();

                    println!("adr: {}, port: {}", adr_and_port[0], adr_and_port[1]);

                    // Split Ipv4 parts
                    let ip: Vec<&str> = adr_and_port[0]
                        .split(".")
                        .collect();
                    // Convert Ipv4 parts to u8
                    let ip_u8: [u8; 4] = [
                        ip[0].parse().unwrap(),
                        ip[1].parse().unwrap(),
                        ip[2].parse().unwrap(),
                        ip[3].parse().unwrap(),
                    ];

                    let port: u16 = adr_and_port[1].parse().expect("Failed to resolve port");

                    let endpoint: Endpoint = Endpoint::new(Ipv4Address::new(ip_u8), port);

                    // Parse args[2] into a Message
                    let message: Message = Message::new(args[2].trim());

                    Ok(TerminalCommand::new(TerminalCommandType::SEND, Some(endpoint), Some(message)))
                }
            },

            "fetch" => {
                if args.len() > 1 {
                    Terminal::usage();
                    Err(TerminalError::TERROR)
                } else {
                    Ok(TerminalCommand::new(TerminalCommandType::FETCH, None, None))
                }
            },

            _ => {
                Terminal::usage();
                Err(TerminalError::TERROR)
            }
        }
    }

    async fn send(
        &self,
        endpoint: Endpoint,
        message: Message,
        machine: Arc<Machine>,
    ) {
        println!("SENDING");

        let transport = self.transport;

        let local_address = match machine.protocol::<DhcpClient>() {
            Some(dhcp) => dhcp.ip_address().await,
            None => self.endpoint.address,
        };

        let endpoints = Endpoints {
            local: Endpoint {
                address: local_address,
                port: 0,
            },
            remote: endpoint,
        };

        let session = transport
            .open_for_sending(self.id(), endpoints, protocols.clone())
            .await
            .unwrap();

        session
            .send(message, protocols.clone())
            .expect("Message failed to send");
    }

    async fn fetch(
        &self,
    ) {
        println!("FETCHING");
        // Print all messages in queue to user terminal
    }

    fn usage() {
        println!(
            "Usage:
            \tsend <ELVIS machine IP w/ Port> <Message>
            \tfetch <-l: only fetch the most recent message>"
        )
    }

    /// Returns and removes the first element in the msg_queue
    fn qpop(
        &self,
    ) -> Option<String> {
        // let mut q: Vec<String> = self.msg_queue
        //     .write()
        //     .unwrap();

        // // Need to test to see what happens with empty queue
        // popped = q.remove(0);

        // match popped {
        //     Some(x) => popped,
        //     None    => println!("No messages in queue!"),
        // }
        None
    }

    /// Adds an element to the end of the msg_queue
    fn qpush(
        &self,
        _msg: String,
    ) {
        // let mut q: Vec<String> = self.msg_queue
        //     .write()
        //     .unwrap();

        // q.push(msg);
    }
}

#[async_trait::async_trait]
impl Protocol for Terminal {
    async fn start(
        &self,
        shutdown: Shutdown,
        _initialize: Arc<Barrier>,
        machine: Arc<Machine>,
    ) -> Result<(), StartError> {
        println!("Start");

        machine
            .protocol::<Udp>()
            .unwrap()
            .listen(self.id(), self.endpoint, machine.clone())
            .unwrap();

        let p = self.port.clone();

        // tokio::spawn(async move {
        // println!("Spawn");
        let listener = TcpListener::bind(p)
            .await
            .unwrap();

        println!("Begin run() on port {}", listener.local_addr().unwrap());

        let (mut socket, _addr) = listener
            .accept()
            .await
            .unwrap();

        let (read, mut write) = socket.split();

        let mut reader = BufReader::new(read);
        let mut line = String::new();

        loop {
            let bytes_read = reader.read_line(&mut line)
                .await
                .unwrap();

            if bytes_read == 0 {
                break;
            }

            let command: TerminalCommand = Terminal::parse(String::from(&line)).unwrap();
            match command.cmd_type {
                TerminalCommandType::SEND => {
                    self.send(
                        command.address.unwrap(),
                        command.message.unwrap(),
                        machine.clone(),
                    ).await;
                },

                TerminalCommandType::FETCH => {
                    self.fetch().await;
                },
            }

            write.write_all(line.as_bytes())
                .await
                .unwrap();

            line.clear();
        }
        
        shutdown.shut_down();

        Ok(())
    }

    fn demux(
        &self,
        message: Message,
        _caller: Arc<dyn Session>,
        _control: Control,
        _protocols: ProtocolMap,
    ) -> Result<(), DemuxError> {
        let msg_as_string = String::from_utf8(message.to_vec()).unwrap();

        println!("{}", msg_as_string);


        Ok(())
    }
}

pub enum TerminalCommandType {
    SEND,
    FETCH,
}

#[derive(Debug)]
pub enum TerminalError {
    TERROR,
}

pub struct TerminalCommand {
    cmd_type: TerminalCommandType,
    message: Option<Message>,
    address: Option<Endpoint>,
}

impl TerminalCommand {
    fn new(
        tct: TerminalCommandType,
        adr: Option<Endpoint>,
        msg: Option<Message>,
    ) -> TerminalCommand {
        Self {
            cmd_type: tct,
            address: adr,
            message: msg,
        }
    }
}
