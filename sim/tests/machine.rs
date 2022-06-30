use elvis::core::{ArcProtocol, Machine};

mod nic_and_capture;

#[test]
pub fn machine() {
    let nic_and_capture::Setup { nic, capture, .. } = nic_and_capture::setup();
    let protocols: [ArcProtocol; 2] = [nic, capture];
    let _machine = Machine::new(protocols.into_iter());
}
