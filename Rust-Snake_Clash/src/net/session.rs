use tokio::sync::mpsc;

use crate::net::messages::{ClientMessage, ServerMessage};

#[derive(Debug, Clone)]
pub struct InboundMessage {
    pub session_id: u64,
    pub message: ClientMessage,
}

#[derive(Debug, Clone)]
pub struct OutboundMessage {
    pub session_id: u64,
    pub message: ServerMessage,
}

#[derive(Debug)]
pub struct SessionHandle {
    pub id: u64,
    pub outbound_tx: mpsc::Sender<ServerMessage>,
}

impl SessionHandle {
    pub fn new(id: u64, outbound_tx: mpsc::Sender<ServerMessage>) -> Self {
        Self { id, outbound_tx }
    }
}
