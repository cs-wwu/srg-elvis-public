//! Types needed for parsing.

use nom::{error::VerboseError, IResult};
use std::collections::HashMap;

/// DecType is the core type of each parse-able item.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum DecType {
    Template,
    Networks,
    Network,
    IP,
    Machines,
    Machine,
    Protocols,
    Protocol,
    Applications,
    Application,
}

pub type Res<T, U> = IResult<T, U, VerboseError<T>>;
pub type Params = HashMap<String, String>;
pub type Networks = HashMap<String, Network>;
pub type MachineNetworks = Vec<MachineNetwork>;
pub type Protocols = Vec<Protocol>;
pub type Applications = Vec<Application>;
pub type Machines = Vec<Machine>;
pub type IPs = Vec<IP>;

/// Interfaces Struct.
/// Holds the various types stored inside a [Machine].
///
///
/// Contains: [MachineNetworks], [Protocols], and [Applications]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Interfaces {
    pub networks: MachineNetworks,
    pub protocols: Protocols,
    pub applications: Applications,
}

/// Machine Struct.
/// Holds core machine info before turning into code
///
///
/// Contains: [DecType], [Params], and the [Interfaces] used for that machine
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Machine {
    pub dectype: DecType,
    pub options: Option<Params>,
    pub interfaces: Interfaces,
}

/// Network Struct.
/// Holds core Network info before turning into code
///
///
/// Contains: [DecType], [Params], and [IPs]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Network {
    pub dectype: DecType,
    pub options: Params,
    pub ip: IPs,
}
/// MachineNetwork Struct.
/// Used to store networks for a machine
///
///
/// Contains the following:
/// [DecType], [Params]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MachineNetwork {
    pub dectype: DecType,
    pub options: Params,
}

/// Protocol Struct.
/// Used to store information for a protocol.
///
///
/// Contains: [DecType] and [Params]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Protocol {
    pub dectype: DecType,
    pub options: Params,
}

/// IP Struct.
/// Used to store IP information for a [Network]
///
///
/// Contains: [DecType] and [Params]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct IP {
    pub dectype: DecType,
    pub options: Params,
}

/// Application Struct.
/// Used to store application info for a [Machine]
///
///
/// Contains: [DecType] and [Params]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Application {
    pub dectype: DecType,
    pub options: Params,
}

/// Sim Struct.
/// Used to store the core parsed Sim.
///
///
/// Contains: [Networks] and [Machines]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Sim {
    pub networks: Networks,
    pub machines: Machines,
}

impl From<&str> for DecType {
    /// Converts a string into a [DecType]
    fn from(i: &str) -> Self {
        match i.to_lowercase().as_str() {
            "template" => DecType::Template,
            "networks" => DecType::Networks,
            "network" => DecType::Network,
            "ip" => DecType::IP,
            "machines" => DecType::Machines,
            "machine" => DecType::Machine,
            "protocols" => DecType::Protocols,
            "protocol" => DecType::Protocol,
            "applications" => DecType::Applications,
            "application" => DecType::Application,
            _ => unimplemented!("No other dec types supported"),
        }
    }
}
