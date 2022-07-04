use elvis::applications::Capture;
use elvis::core::{
    Internet, InternetError, Machine, MachineError, Message, Network, PhysicalAddress, RcProtocol,
};
use elvis::protocols::{Application, Ipv4, Tap};

fn machine() -> Result<Machine, MachineError> {
    let tap = Tap::new_shared(vec![1500]);
    let protocols: [RcProtocol; 2] = [Ipv4::new_shared(), Capture::new_shared()];
    Machine::new(tap, protocols.into_iter())
}

#[test]
pub fn internet() -> Result<(), InternetError> {
    let mut network = Network::new(vec![0, 1], 1500);
    network.send(
        PhysicalAddress::Broadcast,
        Message::new("Hello!").with_header(&Capture::ID.to_bytes()),
    );
    let networks = vec![network];
    let machines = vec![machine()?];
    let mut internet = Internet::new(machines, networks);
    internet.run()?;
    Ok(())
}
