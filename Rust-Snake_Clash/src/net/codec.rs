use crate::net::messages::{
    ClientEnvelope, ClientMessage, ServerEnvelope, ServerMessage, PROTOCOL_VERSION,
};

pub fn encode_server_json(msg: ServerMessage) -> Result<Vec<u8>, serde_json::Error> {
    let env = ServerEnvelope {
        v: PROTOCOL_VERSION,
        msg,
    };
    serde_json::to_vec(&env)
}

pub fn encode_client_json(msg: ClientMessage) -> Result<Vec<u8>, serde_json::Error> {
    let env = ClientEnvelope {
        v: PROTOCOL_VERSION,
        msg,
    };
    serde_json::to_vec(&env)
}

pub fn decode_client_json(bytes: &[u8]) -> Result<ClientMessage, serde_json::Error> {
    let env: ClientEnvelope = serde_json::from_slice(bytes)?;
    Ok(env.msg)
}

pub fn decode_server_json(bytes: &[u8]) -> Result<ServerMessage, serde_json::Error> {
    let env: ServerEnvelope = serde_json::from_slice(bytes)?;
    Ok(env.msg)
}

pub fn encode_server_bin(msg: ServerMessage) -> Result<Vec<u8>, bincode::Error> {
    let env = ServerEnvelope {
        v: PROTOCOL_VERSION,
        msg,
    };
    bincode::serialize(&env)
}

pub fn encode_client_bin(msg: ClientMessage) -> Result<Vec<u8>, bincode::Error> {
    let env = ClientEnvelope {
        v: PROTOCOL_VERSION,
        msg,
    };
    bincode::serialize(&env)
}

pub fn decode_client_bin(bytes: &[u8]) -> Result<ClientMessage, bincode::Error> {
    let env: ClientEnvelope = bincode::deserialize(bytes)?;
    Ok(env.msg)
}

pub fn decode_server_bin(bytes: &[u8]) -> Result<ServerMessage, bincode::Error> {
    let env: ServerEnvelope = bincode::deserialize(bytes)?;
    Ok(env.msg)
}
