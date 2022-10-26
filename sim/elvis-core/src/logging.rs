use tracing::{Level, event};
use tracing_subscriber::{FmtSubscriber};
use std::{sync::Arc};
use std::fs::{OpenOptions, create_dir_all};
use chrono;
use crate::Message;

use super::protocols::ipv4::{Ipv4Address};

/// Logging holds wrapper functions for logging events
/// Each function corresponds to a type of logging (messages, machine creation, etc..)
/// These functions are meant to be called from inside elvis core in the core protocols

//TODO: add more events, add start and end of sim to logs

/// Initializes the event protocol. Only should be called once when the sim starts.
/// Allows for event! to be called and writes to a log file in elvis-core/src/logs.
pub fn init_events(){
    // TODO: Talk to tim abot file paths for cargo testing
    let main_path = "./logs";
    let dir = create_dir_all(main_path);
    match dir {Ok(dir) => dir,Err(error) => panic!("Error: {:?}",error),};
    let file_path = format!("{}/debug-{}.log", main_path, chrono::offset::Local::now().format("%y-%m-%d"));
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
/// Message event handler.
/// Used to log any messages sent. Captures the following data: 
/// local_ip, remote_ip, local_port, remote_port, message_text
pub fn message_event(local_ip: Ipv4Address, remote_ip: Ipv4Address, local_port: u16, remote_port: u16, message: &str){
    event!(target: "MESSAGE", Level::INFO, local_ip = format!("{:?}", local_ip.to_bytes()), remote_ip= format!("{:?}", remote_ip.to_bytes()), local_port= format!("{:x}", local_port), remote_port=format!("{:x}", remote_port), message=message);
}


/// Forward event handler.
/// Used to log any messages Forwarded. Captures the following data: 
/// local_ip, remote_ip, local_port, remote_port, message_text
pub fn forward_event(local_ip: Ipv4Address, remote_ip: Ipv4Address, local_port: u16, remote_port: u16, message: Message){
    // println!("{:#?}", message.iter());
    event!(target: "FORWARD", Level::INFO, local_ip = format!("{:?}", local_ip.to_bytes()), remote_ip= format!("{:?}", remote_ip.to_bytes()), local_port= format!("{:x}", local_port), remote_port=format!("{:x}", remote_port), message=format!("{}", message));
}


/// Capture event handler.
/// Used to log any messages that get "captured" by a machine. Logs:
/// local_ip, local_port, message_text
pub fn capture_event(local_ip: Ipv4Address, local_port: u16,message: Message){
    // println!("{:#?}", message.iter());
    event!(target: "CAPTURE", Level::INFO, local_ip = format!("{:?}", local_ip.to_bytes()), local_port= format!("{:x}", local_port), message=format!("{}", message));
}