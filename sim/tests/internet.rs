use elvis::core::{ArcProtocol, Internet, InternetError, Machine, Network};

mod nic_and_capture;

fn machine() -> Machine {
    let nic_and_capture::Setup { nic, capture, .. } = nic_and_capture::setup();
    let protocols: [ArcProtocol; 2] = [nic, capture];
    Machine::new(protocols.into_iter())
}

#[test]
pub fn internet() -> Result<(), InternetError> {
    let networks = vec![Network::new(vec![0, 1], 1500)];
    let machines = vec![machine(), machine()];
    let mut internet = Internet::new(machines, networks);
    internet.run()?;
    Ok(())
}
