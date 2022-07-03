use elvis::core::{
    Internet, InternetError, Machine, MachineError, Message, Network, PhysicalAddress, RcProtocol,
};
use elvis::protocols::Application;
use tap_and_capture::Capture;

mod tap_and_capture;

fn machine() -> Result<Machine, MachineError> {
    let tap_and_capture::Setup { tap, capture, .. } = tap_and_capture::setup();
    let capture: RcProtocol = capture;
    Machine::new(tap, std::iter::once(capture))
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
