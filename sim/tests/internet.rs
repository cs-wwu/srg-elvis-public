use elvis::{
    core::{ArcProtocol, Internet, InternetError, Machine, Message, Network, PhysicalAddress}, protocols::UserProcess,
};

mod tap_and_capture;

fn machine() -> Machine {
    let tap_and_capture::Setup { tap, capture, .. } = tap_and_capture::setup();
    let capture: ArcProtocol = capture;
    Machine::new(tap, std::iter::once(capture))
}

#[test]
pub fn internet() -> Result<(), InternetError> {
    let mut network = Network::new(vec![0, 1], 1500);
    network.send(
        PhysicalAddress::Broadcast,
        Message::new("Hello!").with_header(&UserProcess::ID.to_bytes()),
    );
    let networks = vec![network];
    let machines = vec![machine()];
    let mut internet = Internet::new(machines, networks);
    internet.run()?;
    Ok(())
}
