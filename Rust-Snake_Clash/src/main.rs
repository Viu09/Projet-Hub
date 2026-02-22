use macroquad::prelude::*;

mod config;
mod client;
mod master;
mod net;
mod state;
mod util;
mod game;

use net::dispatcher::DispatcherHandle;
use net::ws::WsServer;
use state::lobby::Lobby;

fn window_conf() -> Conf {
    Conf {
        window_title: "Snake Clash MVP".to_owned(),
        // Portrait (dev PC mais format mobile)
        window_width: 720,
        window_height: 1280,
        ..Default::default()
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("server") => {
            let lobby = Lobby::new();
            let dispatcher = DispatcherHandle::new(lobby);
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            let _ = rt.block_on(async {
                tokio::spawn(async {
                    let _ = master::serve("0.0.0.0:9100").await;
                });
                let _ = WsServer::serve("0.0.0.0:9001", dispatcher).await;
            });
        }
        Some("master") => {
            let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
            let _ = rt.block_on(async {
                let _ = master::serve("0.0.0.0:9100").await;
            });
        }
        Some("client") | None => {
            macroquad::Window::from_config(window_conf(), game::r#loop::run());
        }
        Some(_) => {
            macroquad::Window::from_config(window_conf(), game::r#loop::run());
        }
    }
}