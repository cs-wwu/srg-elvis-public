use super::{Message, ProtocolContext, Session};
use std::{cell::RefCell, error::Error, rc::Rc};

/// A shared handle to a [`Session`].
///
/// In addition to facilitating multiple ownership, a shared session also acts a
/// proxy to the underlying session and makes sure that the correct current
/// session is applied to the context.
#[derive(Clone)]
pub struct SharedSession {
    session: Rc<RefCell<dyn Session>>,
}

impl SharedSession {
    /// Creates a new shared session
    pub fn new(session: impl Session + 'static) -> Self {
        Self {
            session: Rc::new(RefCell::new(session)),
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
        self.session.borrow_mut().send(message, context)?;
        context.pop_session();
        Ok(())
    }

    /// Updates the current session on the context and calls
    /// [`recv`](Session::recv) on the underlying session.
    pub fn recv(
        &mut self,
        message: Message,
        context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        context.push_session(self.clone());
        self.session.borrow_mut().recv(message, context)?;
        context.pop_session();
        Ok(())
    }

    /// Updates the current session on the context and calls
    /// [`awake`](Session::awake) on the underlying session.
    pub fn awake(&mut self, context: &mut ProtocolContext) -> Result<(), Box<dyn Error>> {
        context.push_session(self.clone());
        self.session.borrow_mut().awake(context)?;
        context.pop_session();
        Ok(())
    }
}

impl From<Rc<RefCell<dyn Session>>> for SharedSession {
    fn from(session: Rc<RefCell<dyn Session>>) -> Self {
        Self { session }
    }
}

impl<T> From<Rc<RefCell<T>>> for SharedSession
where
    T: Session + 'static,
{
    fn from(session: Rc<RefCell<T>>) -> Self {
        Self { session }
    }
}
