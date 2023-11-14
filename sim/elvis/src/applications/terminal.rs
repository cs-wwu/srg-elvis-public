//! The terminal protocol is an application that lives above UDP.
//! The application communicates with an actual port on the real-world machine running ELVIS
//! and sends and receives messages over this port via TCP communication with a command terminal.

use elvis_core::message::Chunk;
use elvis_core::protocols::{Endpoint, Endpoints};
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
    /// The queue of messages received (qpush) by the application that can be
    /// returned (qpop) when a fetch request is made.
    msg_queue: RwLock<Vec<String>>,
    /// The real-world port to communicate with the physical machine's terminal over.
    port: String,
    /// The protocol used to send messages from this machine to other ELVIS machines.
    transport: Transport,
    /// the application's local address
    local_ip: Ipv4Address,
}

impl Terminal {
    pub fn new(
        assign_port: String
    ) -> Terminal {
        Self {
            msg_queue: RwLock::new(Vec::new()),
            port: assign_port,
            transport: Transport::Udp,
            local_ip: Ipv4Address::LOCALHOST,
        }
    }

    async fn run(
        port: String,
        protocols: ProtocolMap,
    ) {
        let listener = TcpListener::bind(port)
            .await
            .unwrap();

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

            // Pass line to TerminalParser to get Message and Endpoint?
            Terminal::parse(String::from(&line));

            write.write_all(line.as_bytes())
                .await
                .unwrap();

            line.clear();
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

                    let adr: u32 = adr_and_port[0].parse().expect("Failed to resolve address");
                    let port: u16 = adr_and_port[1].parse().expect("Failed to resolve port");

                    let endpoint: Endpoint = Endpoint::new(Ipv4Address::from(adr), port);

                    // Parse args[2] into a Message
                    let message: Message = Message::new(args[2]);

                    Ok(TerminalCommand::new(TerminalCommandType::SEND, Some(endpoint), Some(message)))
                }
            },

            "fetch" => {
                Ok(TerminalCommand::new(TerminalCommandType::FETCH, None, None))
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
        protocols: ProtocolMap,
    ) {
        let transport = self.transport;

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
        protocols: ProtocolMap,
    ) -> Result<(), StartError> {
        println!("Begin start()");

        let p = self.port.clone();

        // tokio spawn
        tokio::spawn(async move {
            let listener = TcpListener::bind(p)
                .await
                .unwrap();

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

                // Pass line to TerminalParser to get Message and Endpoint?
                let command: TerminalCommand = Terminal::parse(String::from(&line)).unwrap();
                match command.cmd_type {
                    TerminalCommandType::SEND => {
                        Terminal::send(command.address.unwrap(), command.message.unwrap(), protocols);
                    },

                    TerminalCommandType::FETCH => {

                    },
                }

                write.write_all(line.as_bytes())
                    .await
                    .unwrap();

                line.clear();
            }
            
            shutdown.shut_down();
        });

        Ok(())
    }

    fn demux(
        &self,
        _message: Message,
        _caller: Arc<dyn Session>,
        _control: Control,
        _protocols: ProtocolMap,
    ) -> Result<(), DemuxError> {
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