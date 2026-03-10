use std::{
    collections::HashMap,
    io,
    sync::{Arc, Mutex},
};
use tokio::io::{AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

use crate::protocol::{ClientMsg, ServerMsg};

mod protocol;

const SERVER_ADDR: &str = "127.0.0.1:7878";

// ─── Programme principal ──────────────────────────────────────────────────────────
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let store: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));

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
async fn handle_client(socket: TcpStream, store: Arc<Mutex<HashMap<String, String>>>) {
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
                        value: data.get(&key).cloned(),
                    }
                }

                // Set
                ServerMsg::Set { key, value } => {
                    let mut data = store.lock().unwrap();
                    data.insert(key, value);
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
                        keys: data.keys().into_iter().cloned().collect(),
                    }
                }
                
                ServerMsg::Expire { key: _, seconds: _ } => ClientMsg::Error {
                    status: "error".to_string(),
                    message: "Not yet implemented".to_string(),
                },
                ServerMsg::Ttl { key: _ } => ClientMsg::Error {
                    status: "error".to_string(),
                    message: "Not yet implemented".to_string(),
                },
                ServerMsg::Incr { key: _ } => ClientMsg::Error {
                    status: "error".to_string(),
                    message: "Not yet implemented".to_string(),
                },
                ServerMsg::Decr { key: _ } => ClientMsg::Error {
                    status: "error".to_string(),
                    message: "Not yet implemented".to_string(),
                },
                ServerMsg::Save {} => ClientMsg::Error {
                    status: "error".to_string(),
                    message: "Not yet implemented".to_string(),
                },
            }
        };

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
