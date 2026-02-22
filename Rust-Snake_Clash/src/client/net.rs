use std::sync::{Arc, Mutex};

use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

use crate::net::codec::{
    decode_server_bin, decode_server_json, encode_client_bin, encode_client_json,
};
use crate::net::messages::{ClientMessage, ServerMessage};

#[derive(Clone)]
pub struct ClientRuntime {
    outbound_tx: UnboundedSender<ClientMessage>,
    inbound_rx: Arc<Mutex<UnboundedReceiver<ServerMessage>>>,
}

impl ClientRuntime {
    pub fn connect(url: String) -> Self {
        let (outbound_tx, mut outbound_rx) = unbounded_channel::<ClientMessage>();
        let (inbound_tx, inbound_rx) = unbounded_channel::<ServerMessage>();
        let inbound_rx = Arc::new(Mutex::new(inbound_rx));
        let inbound_handle = inbound_rx.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            rt.block_on(async move {
                let ws_stream = match connect_async(&url).await {
                    Ok((stream, _)) => stream,
                    Err(_) => return,
                };
                let (mut ws_sender, mut ws_receiver) = ws_stream.split();

                loop {
                    tokio::select! {
                        Some(msg) = outbound_rx.recv() => {
                            let payload = encode_client_bin(msg.clone())
                                .or_else(|_| encode_client_json(msg));
                            if let Ok(bytes) = payload {
                                let _ = ws_sender.send(Message::Binary(bytes)).await;
                            }
                        }
                        inbound = ws_receiver.next() => {
                            match inbound {
                                Some(Ok(Message::Binary(bytes))) => {
                                    let msg = decode_server_bin(&bytes)
                                        .or_else(|_| decode_server_json(&bytes));
                                    if let Ok(msg) = msg {
                                        let _ = inbound_tx.send(msg);
                                    }
                                }
                                Some(Ok(Message::Text(text))) => {
                                    if let Ok(msg) = decode_server_json(text.as_bytes()) {
                                        let _ = inbound_tx.send(msg);
                                    }
                                }
                                Some(Ok(Message::Close(_))) | None | Some(Err(_)) => {
                                    break;
                                }
                                _ => {}
                            }
                        }
                    }
                }
            });
        });

        Self {
            outbound_tx,
            inbound_rx: inbound_handle,
        }
    }

    pub fn send(&self, msg: ClientMessage) {
        let _ = self.outbound_tx.send(msg);
    }

    pub fn try_recv(&self) -> Option<ServerMessage> {
        self.inbound_rx.lock().ok()?.try_recv().ok()
    }
}
