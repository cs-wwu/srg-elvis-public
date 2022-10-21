use tracing::{Level, event};
use tracing_subscriber::{FmtSubscriber};
use std::{sync::Arc};
use std::fs::{OpenOptions};
use chrono;
use super::protocols::ipv4::{Ipv4Address};

/// Logging holds wrapper functions for logging events
/// Each function corresponds to a type of logging (messages, machine creation, etc..)
/// These functions are meant to be called from inside elvis core in the core protocols

//TODO: add more events, add start and end of sim to logs

/// Initializes the event protocol. Only should be called once when the sim starts.
/// Allows for event! to be called and writes to a log file in elvis-core/src/logs.
pub fn init_events(){
    let file_path = format!("elvis-core/src/logs/debug-{}.log", chrono::offset::Local::now().format("%y-%m-%d"));
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
    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
}

/// Message event handler.
/// Used to log any messages sent. Captures the following data: 
/// local_ip, remote_ip, local_port, remote_port, message_text
pub fn message_event(local_ip: Ipv4Address, remote_ip: Ipv4Address, local_port: u16, remote_port: u16, message: &str){
    event!(target: "MESSAGE", Level::INFO, local_ip = format!("{:?}", local_ip.to_bytes()), remote_ip= format!("{:?}", remote_ip.to_bytes()), local_port= format!("{:x}", local_port), remote_port=format!("{:x}", remote_port), message=message);
}