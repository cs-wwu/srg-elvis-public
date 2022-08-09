use super::{Message, ProtocolContext, Session};
use std::{
    error::Error,
    sync::{Arc, Mutex},
};

/// A shared handle to a [`Session`].
///
/// In addition to facilitating multiple ownership, a shared session also acts a
/// proxy to the underlying session and makes sure that the correct current
/// session is applied to the context.
#[derive(Clone)]
pub struct SharedSession {
    session: Arc<Mutex<dyn Session + Send + Sync>>,
}

impl SharedSession {
    /// Creates a new shared session
    pub fn new(session: impl Session + Send + Sync + 'static) -> Self {
        Self {
            session: Arc::new(Mutex::new(session)),
        }
    }

    /// Updates the current session on the context and calls
    /// [`send`](Session::send) on the underlying session.
    pub fn send(
        &mut self,
        message: Message,
        context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        context.push_session(self.clone());
        self.session.lock().unwrap().send(message, context)?;
        context.pop_session();
        Ok(())
    }

    /// Updates the current session on the context and calls
    /// [`receive`](Session::receive) on the underlying session.
    pub fn receive(
        &mut self,
        message: Message,
        context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        context.push_session(self.clone());
        self.session.lock().unwrap().receive(message, context)?;
        context.pop_session();
        Ok(())
    }
}

impl From<Arc<Mutex<dyn Session + Send + Sync>>> for SharedSession {
    fn from(session: Arc<Mutex<dyn Session + Send + Sync>>) -> Self {
        Self { session }
    }
}

impl<T> From<Arc<Mutex<T>>> for SharedSession
where
    T: Session + Send + Sync + 'static,
{
    fn from(session: Arc<Mutex<T>>) -> Self {
        Self { session }
    }
}
