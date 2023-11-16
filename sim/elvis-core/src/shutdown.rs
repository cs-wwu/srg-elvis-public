use tokio::sync::{broadcast, mpsc};

#[derive(Debug, Clone)]
pub struct Shutdown {
    /// This channel can be used tell the simulation to shutdown or wait for the
    /// simulation shutdown message
    notify: broadcast::Sender<ExitStatus>,

    /// The internet will wait for all Shutdown instances to be dropped using
    /// this channel.
    #[allow(dead_code)]
    confirm: mpsc::Sender<ExitStatus>,
}

impl Shutdown {
    pub fn new() -> Self {
        let (notify, _) = broadcast::channel(1);
        let (confirm, _) = mpsc::channel(1);
        Self { notify, confirm }
    }

    pub fn shut_down(&self) {
        if let Err(e) = self.notify.send(ExitStatus::Exited) {
            tracing::error!("Failed to initiate shutdown: {}", e);
        }
    }

    pub fn shut_down_with_status(&self, status: ExitStatus) {
        if let Err(e) = self.notify.send(status.clone()) {
            tracing::error!("Failed to initiate shutdown: {}", e);
        }
    }

    pub fn receiver(&self) -> broadcast::Receiver<ExitStatus> {
        self.notify.subscribe()
    }
}

impl Default for Shutdown {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ExitStatus {
    Status(u32),
    Exited,
    TimedOut,
}
