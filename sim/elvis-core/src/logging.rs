//! Contains basic logging functions.

use tracing::{Level, event};
use tracing_subscriber::{FmtSubscriber};
use std::{sync::Arc};
use std::fs::{OpenOptions, create_dir_all};
use chrono;
use crate::Message;
use crate::protocol::ProtocolId;

use super::protocols::ipv4::{Ipv4Address};

/// Logging holds wrapper functions for logging events
/// Each function corresponds to a type of logging (messages, machine creation, etc..)
/// These functions are meant to be called from inside elvis core in the core protocols
/// Messages will be logged as Bytes in Hex formatting for most convenient parsing

/// Initializes the event protocol. Only should be called once when the sim starts.
/// Allows for event! to be called and writes to a log file in elvis-core/src/logs.
/// During Tests -- cargo test -- logs will not be generated for the time being
pub fn init_events(){
    let main_path = "./logs";
    let dir = create_dir_all(main_path);
    match dir {Ok(dir) => dir,Err(error) => panic!("Error: {:?}",error),};
    let file_path = format!("{}/debug-{}.log", main_path, chrono::offset::Local::now().format("%y-%m-%d_%H-%M-%S"));
    let file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(file_path);
    let file = match file  {Ok(file) => file,Err(error) => panic!("Error: {:?}",error),};
    let subscriber = FmtSubscriber::builder()
        .with_writer(Arc::new(file))
        .json()
        .finish();
    // set the global default so all events/logs go to the same subscriber and subsequently the same file
    // TODO: Talk to tim on handling errors properly
    match tracing::subscriber::set_global_default(subscriber){
        Ok(sub) => sub,
        Err(error) => println!("{:?}", error),
    };
    
}
/// Send message event handler.
/// Used to log any messages sent. Captures the following data: 
/// local_ip, remote_ip, local_port, remote_port, message_text
pub fn send_message_event(local_ip: Ipv4Address, remote_ip: Ipv4Address, local_port: u16, remote_port: u16, message: Message){
    event!(target: "SEND_MESSAGE", Level::INFO, local_ip = format!("{:?}", local_ip.to_bytes()), remote_ip= format!("{:?}", remote_ip.to_bytes()), local_port= format!("{:x}", local_port), remote_port=format!("{:x}", remote_port), message=format!("{}", message));
}

/// Receive message event handler.
/// Used to log any messages received. Captures the following data: 
/// local_ip, remote_ip, local_port, remote_port, message_text
pub fn receive_message_event(local_ip: Ipv4Address, remote_ip: Ipv4Address, local_port: u16, remote_port: u16, message: Message){
    event!(target: "RECV_MESSAGE", Level::INFO, local_ip = format!("{:?}", local_ip.to_bytes()), remote_ip= format!("{:?}", remote_ip.to_bytes()), local_port= format!("{:x}", local_port), remote_port=format!("{:x}", remote_port), message=format!("{}", message));
}


// TODO: correlate the machine id's to IP's or protocols
/// Machine creation event handler.
/// Used to log the creation of any machines added to the sim. Will log:
/// machine_id, list of all protocol id's associated with the machine
/// This will eventually contain more info as the simulation evolves
pub fn machine_creation_event(machine_id: usize, protocol_ids: Vec<ProtocolId>){
    event!(target: "MACHINE_CREATION", Level::INFO, machine_id=machine_id, protocol_ids = format!("{:?}", protocol_ids));
}