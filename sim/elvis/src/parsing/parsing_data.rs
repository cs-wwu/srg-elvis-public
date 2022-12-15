//! Types needed for parsing.

use nom::{
    error::{VerboseError},
    IResult
};

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
pub type Param<'a> = (&'a str, &'a str);
pub type Params<'a> = Vec<Param<'a>>;
pub type Networks<'a> = Vec<Network<'a>>;
pub type MachineNetworks<'a> = Vec<MachineNetwork<'a>>;
pub type Protocols<'a> = Vec<Protocol<'a>>;
pub type Applications<'a> = Vec<Application<'a>>;
pub type Machines<'a> = Vec<Machine<'a>>;
pub type IPs<'a> = Vec<IP<'a>>;

/// Interfaces Struct.
/// Holds the various types stored inside a [Machine].
/// 
/// 
/// Contains: [MachineNetworks], [Protocols], and [Applications]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Interfaces<'a> {
    pub networks: MachineNetworks<'a>,
    pub protocols: Protocols<'a>,
    pub applications: Applications<'a>,
}

/// Machine Struct.
/// Holds core machine info before turning into code
///
/// 
/// Contains: [DecType], [Params], and the [Interfaces] used for that machine
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Machine<'a> {
    pub dectype: DecType,
    pub options: Option<Params<'a>>,
    pub interfaces: Interfaces<'a>
}

/// Network Struct.
/// Holds core Network info before turning into code
/// 
/// 
/// Contains: [DecType], [Params], and [IPs]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Network<'a> {
    pub dectype: DecType,
    pub options: Params<'a>,
    pub ip: IPs<'a>,
}
/// MachineNetwork Struct.
/// Used to store networks for a machine
/// 
/// 
/// Contains the following:
/// [DecType], [Params]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct MachineNetwork<'a> {
    pub dectype: DecType,
    pub options: Params<'a>,
}

/// Protocol Struct.
/// Used to store information for a protocol.
/// 
/// 
/// Contains: [DecType] and [Params]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Protocol<'a> {
    pub dectype: DecType,
    pub options: Params<'a>
}

/// IP Struct.
/// Used to store IP information for a [Network]
/// 
/// 
/// Contains: [DecType] and [Params]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct IP<'a> {
    pub dectype: DecType,
    pub options: Params<'a>
}

/// Application Struct.
/// Used to store application info for a [Machine]
/// 
/// 
/// Contains: [DecType] and [Params]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Application<'a> {
    pub dectype: DecType,
    pub options: Params<'a>
}

/// Sim Struct.
/// Used to store the core parsed Sim.
/// 
/// 
/// Contains: [Networks] and [Machines]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Sim<'a> {
    pub networks: Networks<'a>,
    pub machines: Machines<'a>
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