use tokio::sync::broadcast;

/// A struct which can be used to shut down a simulation.
/// You can create multiple connected shutdowns by cloning.
#[derive(Debug, Clone)]
pub struct Shutdown {
    /// This channel can be used tell the simulation to shut down.
    notify: broadcast::Sender<ExitStatus>,
    /// Keeps track of the last status received
    /// So users can call `wait_for_shutdown` multiple times
    last_status: Option<ExitStatus>,
}

impl Shutdown {
    /// Creates a new active shutdown.
    pub fn new() -> Self {
        let (notify, _) = broadcast::channel(1);
        Self {
            notify,
            last_status: None,
        }
    }

    /// Sends `ExitStatus::Exited` to all `Shutdowns` cloned from this one.
    pub fn shut_down(&self) {
        if let Err(e) = self.notify.send(ExitStatus::Exited) {
            tracing::error!("Failed to initiate shutdown: {}", e);
        }
    }

    /// Sends `status` to all `Shutdowns` cloned from this one.
    pub fn shut_down_with_status(&self, status: ExitStatus) {
        if let Err(e) = self.notify.send(status.clone()) {
            tracing::error!("Failed to initiate shutdown: {}", e);
        }
    }

    /// Waits to receive a shutdown status.
    pub async fn wait_for_shutdown(&mut self) -> ExitStatus {
        use tokio::sync::broadcast::error::RecvError;

        if let Some(status) = self.last_status {
            return status;
        } else {
            let mut recv = self.notify.subscribe();
        
            loop {
                match recv.recv().await {
                    Ok(status) => {
                        self.last_status = Some(status);
                        return status;
                    },
                    Err(RecvError::Closed) => unreachable!(),
                    Err(RecvError::Lagged(_)) => (),
                }
            };
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
