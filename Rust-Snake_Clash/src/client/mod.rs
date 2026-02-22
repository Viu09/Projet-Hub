#![allow(dead_code)]

pub mod net;
pub mod predict;
pub mod state;
pub mod runtime;
pub mod menu;
pub mod master_api;
pub mod lobby_ui;

#[allow(dead_code)]
pub struct ClientConfig {
    pub server_url: String,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            server_url: "ws://127.0.0.1:9001".to_owned(),
        }
    }
}
