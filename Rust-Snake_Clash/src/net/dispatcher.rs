use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{mpsc, Mutex};

use crate::net::messages::ServerMessage;
use crate::net::session::{InboundMessage, OutboundMessage, SessionHandle};
use crate::state::lobby::Lobby;

#[derive(Clone)]
pub struct DispatcherHandle {
    inner: Arc<Mutex<Dispatcher>>,
}

impl DispatcherHandle {
    pub fn new(lobby: Lobby) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Dispatcher::new(lobby))),
        }
    }

    pub async fn register_session(&self, session: SessionHandle) {
        let mut guard = self.inner.lock().await;
        guard.sessions.insert(session.id, session.outbound_tx);
    }

    pub async fn unregister_session(&self, session_id: u64) {
        let outbound = {
            let mut guard = self.inner.lock().await;
            guard.sessions.remove(&session_id);
            guard.lobby.handle_disconnect(session_id)
        };

        for msg in outbound {
            self.send_outbound(msg).await;
        }
    }

    pub async fn handle_inbound(&self, inbound: InboundMessage) -> Vec<OutboundMessage> {
        let mut guard = self.inner.lock().await;
        guard.lobby.handle_message(inbound.session_id, inbound.message)
    }

    pub async fn send_outbound(&self, outbound: OutboundMessage) {
        let guard = self.inner.lock().await;
        if let Some(tx) = guard.sessions.get(&outbound.session_id) {
            let _ = tx.send(outbound.message).await;
        }
    }

    pub async fn tick(&self) -> Vec<OutboundMessage> {
        let mut guard = self.inner.lock().await;
        guard.lobby.tick()
    }
}

struct Dispatcher {
    lobby: Lobby,
    sessions: HashMap<u64, mpsc::Sender<ServerMessage>>,
}

impl Dispatcher {
    fn new(lobby: Lobby) -> Self {
        Self {
            lobby,
            sessions: HashMap::new(),
        }
    }
}
