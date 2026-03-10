use std::{
    collections::HashMap,
    io,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::io::{AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::interval;

use crate::protocol::{ClientMsg, ServerMsg};

mod protocol;

const SERVER_ADDR: &str = "127.0.0.1:7878";
struct ServerEntry {
    expire: Option<Instant>,
    value: String,
}

// ─── Programme principal ──────────────────────────────────────────────────────────
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let store: Arc<Mutex<HashMap<String, ServerEntry>>> = Arc::new(Mutex::new(HashMap::new()));

    // Tâche de fond : nettoie les clés expirées toutes les secondes
    let store_cleanup = store.clone();
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(1));
        loop {
            ticker.tick().await;
            let mut data = store_cleanup.lock().unwrap();
            data.retain(|_, entry| entry.expire.map(|exp| exp > Instant::now()).unwrap_or(true));
        }
    });

    let listener = TcpListener::bind(SERVER_ADDR)
        .await
        .expect("Failed to bind TCP listener");

    println!("Le serveur est lancé sur {} !", SERVER_ADDR);

    loop {
        let (socket, _addr) = match listener.accept().await {
            Ok(accept) => accept,
            Err(e) => {
                tracing::error!("Erreur lors de l'acceptation de la connexion: {}", e);
                continue;
            }
        };
        let store = store.clone();
        tokio::spawn(async move {
            handle_client(socket, store).await;
        });
    }
}

// ─── Client ──────────────────────────────────────────────────────────
#[allow(unused_variables)]
async fn handle_client(socket: TcpStream, store: Arc<Mutex<HashMap<String, ServerEntry>>>) {
    let (read_half, mut write_half) = socket.into_split();
    let mut reader = BufReader::new(read_half);
    let mut line = String::new();

    loop {
        // Lire les lignes
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break,
            Err(e) => {
                tracing::error!("Erreur de lecture: {}", e);
                continue;
            }
            Ok(_) => {}
        }

        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let response = 'server_response: {
            // Parse du message
            let msg: ServerMsg = match serde_json::from_str(line) {
                Ok(m) => m,
                Err(_) => {
                    break 'server_response ClientMsg::Error {
                        status: "error".to_string(),
                        message: "invalid json".to_string(),
                    };
                }
            };

            // Création de la réponse
            match msg {
                // Ping
                ServerMsg::Ping {} => ClientMsg::Ping {
                    status: "ok".to_string(),
                },

                // Get
                ServerMsg::Get { key } => {
                    let data = store.lock().unwrap();
                    ClientMsg::Get {
                        status: "ok".to_string(),
                        value: data.get(&key).map(|entry| entry.value.clone()),
                    }
                }

                // Set
                ServerMsg::Set { key, value } => {
                    let mut data = store.lock().unwrap();
                    data.insert(
                        key,
                        ServerEntry {
                            value,
                            expire: None,
                        },
                    );
                    ClientMsg::Set {
                        status: "ok".to_string(),
                    }
                }

                // Del
                ServerMsg::Del { key } => {
                    let mut data = store.lock().unwrap();
                    ClientMsg::Del {
                        status: "ok".to_string(),
                        count: data.remove(&key).map_or(0, |val| 1),
                    }
                }

                // Keys
                ServerMsg::Keys {} => {
                    let data = store.lock().unwrap();
                    ClientMsg::Keys {
                        status: "ok".to_string(),
                        keys: data.keys().cloned().collect(),
                    }
                }

                // Expire
                ServerMsg::Expire { key, seconds } => {
                    let mut data = store.lock().unwrap();
                    if let Some(entry) = data.get_mut(&key) {
                        entry.expire = Some(Instant::now() + Duration::from_secs(seconds as u64));
                    }
                    ClientMsg::Expire {
                        status: "ok".to_string(),
                    }
                }

                // TTL
                ServerMsg::Ttl { key } => {
                    let data = store.lock().unwrap();
                    let ttl: i64 = match data.get(&key) {
                        None => -2,
                        Some(entry) => match &entry.expire {
                            None => -1,
                            Some(expire) => {
                                expire.saturating_duration_since(Instant::now()).as_secs() as i64
                            }
                        },
                    };
                    ClientMsg::Ttl {
                        status: "ok".to_string(),
                        ttl,
                    }
                }

                // Incr
                ServerMsg::Incr { key } => {
                    let mut data = store.lock().unwrap();
                    let (val, ttl) = match data.get(&key) {
                        Some(entry) => match entry.value.trim().parse::<i64>() {
                            Ok(v) => (v + 1, entry.expire),
                            Err(_) => {
                                break 'server_response ClientMsg::Error {
                                    status: "error".to_string(),
                                    message: "not an integer".to_string(),
                                };
                            }
                        },
                        None => (1, None),
                    };
                    data.insert(
                        key,
                        ServerEntry {
                            value: val.to_string(),
                            expire: ttl,
                        },
                    );
                    ClientMsg::Incr {
                        status: "ok".to_string(),
                        value: val,
                    }
                }

                // Decr
                ServerMsg::Decr { key } => {
                    let mut data = store.lock().unwrap();
                    let (val, ttl) = match data.get(&key) {
                        Some(entry) => match entry.value.trim().parse::<i64>() {
                            Ok(v) => (v - 1, entry.expire),
                            Err(_) => {
                                break 'server_response ClientMsg::Error {
                                    status: "error".to_string(),
                                    message: "not an integer".to_string(),
                                };
                            }
                        },
                        None => (-1, None),
                    };
                    data.insert(
                        key,
                        ServerEntry {
                            value: val.to_string(),
                            expire: ttl,
                        },
                    );
                    ClientMsg::Decr {
                        status: "ok".to_string(),
                        value: val,
                    }
                }

                // Save
                ServerMsg::Save {} => ClientMsg::Error {
                    status: "error".to_string(),
                    message: "Not yet implemented".to_string(),
                },
            }
        };

        // Envoyer la réponse
        if send_response(&mut write_half, &response).await.is_err() {
            return;
        }
    }
}

async fn send_response<W>(writer: &mut W, msg: &ClientMsg) -> io::Result<()>
where
    W: AsyncWrite + Unpin,
{
    let json = serde_json::to_string(msg).expect("sérialisation échouée");
    writer.write_all(json.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    Ok(())
}
