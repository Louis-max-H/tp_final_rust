use serde::{Deserialize, Serialize};

// ─── Messages envoyés par le serveur ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "cmd", content = "data")]
pub enum ServerMsg {
    /// Premier message reçu après connexion.
    Ping {},
    Get {
        key: String,
    },
    Set {
        key: String,
        value: String,
    },
    Del {
        key: String,
    },
    Keys {},
    Expire {
        key: String,
        seconds: usize,
    },
    Ttl {
        key: String,
    },
    Incr {
        key: String,
    },
    Decr {
        key: String,
    },
    Save {},
}

// ─── Messages envoyés par le client ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ClientMsg {
    /// Premier message reçu après connexion.
    Ping {
        status: String,
    },
    Get {
        status: String,
        value: Option<String>,
    },
    Set {
        status: String,
    },
    Del {
        status: String,
        count: usize,
    },
    Keys {
        status: String,
        keys: Vec<String>,
    },
    Expire {
        status: String,
    },
    Ttl {
        status: String,
        ttl: usize,
    },
    Incr {
        status: String,
        value: i64,
    },
    Decr {
        status: String,
        value: i64,
    },
    Save {
        status: String,
    },
    Error {
        status: String,
        message: String,
    },
}
