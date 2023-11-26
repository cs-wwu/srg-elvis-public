use std::sync::{OnceLock, Arc};

use tokio::sync::broadcast;

/// A struct which can be used to shut down a simulation.
/// You can create multiple connected shutdowns by cloning.
#[derive(Debug, Clone)]
pub struct Shutdown {
    /// The exit status.
    exit_status: Arc<OnceLock<ExitStatus>>,
    /// This channel is sent on when the exit status is set.
    notify: broadcast::Sender<()>,
}

impl Shutdown {
    /// Creates a new active shutdown.
    pub fn new() -> Self {
        let (notify, recv) = broadcast::channel(1);
        Self {
            exit_status: Arc::new(OnceLock::new()),
            notify,
        }
    }

    /// Sends `ExitStatus::Exited` to all `Shutdowns` cloned from this one.
    pub fn shut_down(&self) {
        self.shut_down_with_status(ExitStatus::Exited);
    }

    /// Sends `status` to all `Shutdowns` cloned from this one.
    /// If a shutdown has already occured, nothing happens.
    pub fn shut_down_with_status(&self, status: ExitStatus) {
        self.exit_status.set(status);
        let _ = self.notify.send(());
    }

    /// Returns an ExitStatus if a shutdown status has been sent already,
    /// or None if a shutdown has not happened yet.
    pub fn try_get_status(&self)  -> Option<ExitStatus> {
        self.exit_status.get().copied()
    }

    /// Waits to receive a shutdown status.
    pub async fn wait_for_shutdown(&self) -> ExitStatus {
        let mut recv = self.notify.subscribe();

        loop {
            match self.try_get_status() {
                Some(status) => return status,
                None => _ = recv.recv().await,
            }
        }
    }
}

impl Default for Shutdown {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ExitStatus {
    Status(u32),
    Exited,
    TimedOut,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_active() {
        let status = ExitStatus::Status(22);
        let shut0 = Shutdown::new();
        let mut shuts = [shut0.clone(), shut0.clone(), shut0.clone()];

        shuts[0].shut_down_with_status(status);

        for shut in shuts {
            assert_eq!(shut.wait_for_shutdown().await, status);
        }
    }
}
