use crate::{
    control::{Key, Primitive},
    protocol::Context,
    Message, Session,
};
use std::{error::Error, sync::Arc};

pub struct TcpSession {}

impl Session for TcpSession {
    fn send(self: Arc<Self>, message: Message, context: Context) -> Result<(), Box<dyn Error>> {
        todo!()
    }

    fn receive(self: Arc<Self>, message: Message, context: Context) -> Result<(), Box<dyn Error>> {
        todo!()
    }

    fn query(self: Arc<Self>, key: Key) -> Result<Primitive, Box<dyn Error>> {
        todo!()
    }
}
