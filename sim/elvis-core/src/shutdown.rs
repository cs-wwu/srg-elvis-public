use tokio::sync::broadcast;

#[derive(Debug, Clone)]
pub struct Shutdown(broadcast::Sender<()>);

impl Shutdown {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(1);
        Self(tx)
    }

    pub fn signal_shutdown(&self) {
        let _ = self.0.send(());
    }

    pub fn rx(&self) -> broadcast::Receiver<()> {
        self.0.subscribe()
    }
}
