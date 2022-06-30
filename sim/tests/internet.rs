use elvis::core::{
    ArcProtocol, Internet, InternetError, Machine, Message, Network, PhysicalAddress,
};

mod nic_and_capture;

fn machine() -> Machine {
    let nic_and_capture::Setup { nic, capture, .. } = nic_and_capture::setup();
    let capture: ArcProtocol = capture;
    Machine::new(nic, std::iter::once(capture))
}

#[test]
pub fn internet() -> Result<(), InternetError> {
    let mut network = Network::new(vec![0, 1], 1500);
    network.send(PhysicalAddress::Broadcast, Message::new("Hello!"));
    let networks = vec![network];
    let machines = vec![machine()];
    let mut internet = Internet::new(machines, networks);
    internet.run()?;
    Ok(())
}
