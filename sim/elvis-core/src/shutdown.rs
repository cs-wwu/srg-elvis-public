use tokio::sync::{broadcast, mpsc};

#[derive(Debug, Clone)]
pub struct Shutdown {
    /// This channel can be used tell the simulation to shutdown or wait for the
    /// simulation shutdown message
    notify: broadcast::Sender<()>,

    /// The internet will wait for all Shutdown instances to be dropped using
    /// this channel.
    #[allow(dead_code)]
    confirm: mpsc::Sender<()>,
}

impl Shutdown {
    pub fn new() -> (Self, mpsc::Receiver<()>) {
        let (notify, _) = broadcast::channel(1);
        let (confirm_tx, confirm_rx) = mpsc::channel(1);
        (
            Self {
                notify,
                confirm: confirm_tx,
            },
            confirm_rx,
        )
    }

    pub fn shut_down(&self) {
        let _ = self.notify.send(());
    }

    pub fn receiver(&self) -> broadcast::Receiver<()> {
        self.notify.subscribe()
    }
}
