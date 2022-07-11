use super::{Message, ProtocolContext, Session};
use std::{cell::RefCell, error::Error, rc::Rc};

#[derive(Clone)]
pub struct SharedSession {
    session: Rc<RefCell<dyn Session>>,
}

impl SharedSession {
    pub fn new(session: impl Session + 'static) -> Self {
        Self {
            session: Rc::new(RefCell::new(session)),
        }
    }

    pub fn send(
        &mut self,
        message: Message,
        context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        context.current_session = Some(self.clone());
        self.session.borrow_mut().send(message, context)?;
        context.current_session = None;
        Ok(())
    }

    pub fn recv(
        &mut self,
        message: Message,
        context: &mut ProtocolContext,
    ) -> Result<(), Box<dyn Error>> {
        context.current_session = Some(self.clone());
        self.session.borrow_mut().recv(message, context)?;
        context.current_session = None;
        Ok(())
    }

    pub fn awake(&mut self, context: &mut ProtocolContext) -> Result<(), Box<dyn Error>> {
        context.current_session = Some(self.clone());
        self.session.borrow_mut().awake(context)?;
        context.current_session = None;
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
