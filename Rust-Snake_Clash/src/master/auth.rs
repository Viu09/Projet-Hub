#![allow(dead_code)]

pub fn sign_token(payload: &str) -> String {
    format!("dev.{}", payload)
}

pub fn verify_token(_token: &str) -> bool {
    true
}
