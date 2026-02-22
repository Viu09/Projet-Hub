use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::accept_async;

use crate::net::codec::{
    decode_client_bin, decode_client_json, encode_server_bin, encode_server_json,
};
use crate::net::dispatcher::DispatcherHandle;
use crate::net::session::{InboundMessage, SessionHandle};

pub struct WsServer;

impl WsServer {
    pub async fn serve(addr: &str, dispatcher: DispatcherHandle) -> tokio::io::Result<()> {
        let listener = TcpListener::bind(addr).await?;
        let mut next_id: u64 = 1;

        let tick_dispatcher = dispatcher.clone();
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_millis(50));
            loop {
                ticker.tick().await;
                let outbound = tick_dispatcher.tick().await;
                for msg in outbound {
                    tick_dispatcher.send_outbound(msg).await;
                }
            }
        });

        loop {
            let (stream, _) = listener.accept().await?;
            let dispatcher = dispatcher.clone();
            let session_id = next_id;
            next_id = next_id.saturating_add(1);

            tokio::spawn(async move {
                let ws_stream = match accept_async(stream).await {
                    Ok(stream) => stream,
                    Err(_) => return,
                };
                let (mut ws_sender, mut ws_receiver) = ws_stream.split();
                let (outbound_tx, mut outbound_rx) = mpsc::channel(64);

                dispatcher
                    .register_session(SessionHandle::new(session_id, outbound_tx))
                    .await;

                loop {
                    tokio::select! {
                        inbound = ws_receiver.next() => {
                            match inbound {
                                Some(Ok(Message::Text(text))) => {
                                    if let Ok(msg) = decode_client_json(text.as_bytes()) {
                                        let outbound = dispatcher.handle_inbound(InboundMessage {
                                            session_id,
                                            message: msg,
                                        }).await;
                                        for out in outbound {
                                            dispatcher.send_outbound(out).await;
                                        }
                                    }
                                }
                                Some(Ok(Message::Binary(bytes))) => {
                                    let msg = decode_client_bin(&bytes)
                                        .or_else(|_| decode_client_json(&bytes));
                                    if let Ok(msg) = msg {
                                        let outbound = dispatcher.handle_inbound(InboundMessage {
                                            session_id,
                                            message: msg,
                                        }).await;
                                        for out in outbound {
                                            dispatcher.send_outbound(out).await;
                                        }
                                    }
                                }
                                Some(Ok(Message::Close(_))) | None | Some(Err(_)) => {
                                    break;
                                }
                                _ => {}
                            }
                        }
                        outbound = outbound_rx.recv() => {
                            if let Some(msg) = outbound {
                                let payload = encode_server_bin(msg.clone())
                                    .or_else(|_| encode_server_json(msg));
                                if let Ok(payload) = payload {
                                    let _ = ws_sender.send(Message::Binary(payload)).await;
                                }
                            } else {
                                break;
                            }
                        }
                    }
                }

                dispatcher.unregister_session(session_id).await;
            });
        }
    }
}
