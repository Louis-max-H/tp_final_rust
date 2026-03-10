use std::{
    collections::HashMap,
    error::Error,
    sync::{Arc, Mutex},
    thread::{self, sleep},
    time::Duration,
};
use tokio::net::{TcpListener, TcpStream};
use tungstenite::{connect, Message};

use crate::protocol::{ClientMsg, ServerMsg};

mod protocol;

const SERVER_URL: &str = "127.0.0.1:7878";

#[tokio::main]
async fn main() {
    // Initialiser tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // TODO: Implémenter le serveur MiniRedis sur 127.0.0.1:7878

    // Étapes suggérées :
    // 1. Créer le store partagé (Arc<Mutex<HashMap<String, ...>>>)
    let store: Arc<Mutex<HashMap<String, i64>>> = Arc::new(Mutex::new(HashMap::new()));

    // 2. Bind un TcpListener sur 127.0.0.1:7878
    let listener = TcpListener::bind(SERVER_URL)
        .await
        .expect("Failed to bind TCP listener");

    // 3. Accept loop : pour chaque connexion, spawn une tâche
    loop {
        let (socket, store) = match listener.accept().await {
            Ok((socket, _addr)) => (socket, store.clone()),
            Err(e) => {
                tracing::error!("Erreur lors de l'acceptation de la connexion: {}", e);
                continue;
            }
        };
        tokio::spawn(async move {
            handle_client(socket, store).await;
        });
    }
    // 4. Dans chaque tâche : lire les requêtes JSON ligne par ligne,
    //    traiter la commande, envoyer la réponse JSON + '\n'
}

async fn handle_client(socket: TcpStream, store: Arc<Mutex<HashMap<String, i64>>>) {
    let (mut ws, _response) = connect(SERVER_URL).expect("impossible de se connecter au serveur");

    loop {
        let msg: ServerMsg = match read_server_msg(&mut ws) {
            Err(e) => {
                tracing::error!("Erreur lors de la lecture du message: {}", e);
                return;
            }
            Ok(None) => {
                thread::sleep(Duration::from_millis(50));
                continue;
            }
            Ok(Some(msg)) => msg,
        };

        let response = match msg {
            // Ping
            ServerMsg::Ping {} => ClientMsg::Ping {
                status: "ok".to_string(),
            },

            ServerMsg::Get { key } => ClientMsg::Error {
                status: "error".to_string(),
                message: "Not yet implemented".to_string(),
            },
            ServerMsg::Set { key, value } => ClientMsg::Error {
                status: "error".to_string(),
                message: "Not yet implemented".to_string(),
            },
            ServerMsg::Del { key } => ClientMsg::Error {
                status: "error".to_string(),
                message: "Not yet implemented".to_string(),
            },
            ServerMsg::Keys {} => ClientMsg::Error {
                status: "error".to_string(),
                message: "Not yet implemented".to_string(),
            },
            ServerMsg::Expire { key, seconds } => ClientMsg::Error {
                status: "error".to_string(),
                message: "Not yet implemented".to_string(),
            },
            ServerMsg::Ttl { key } => ClientMsg::Error {
                status: "error".to_string(),
                message: "Not yet implemented".to_string(),
            },
            ServerMsg::Incr { key } => ClientMsg::Error {
                status: "error".to_string(),
                message: "Not yet implemented".to_string(),
            },
            ServerMsg::Decr { key } => ClientMsg::Error {
                status: "error".to_string(),
                message: "Not yet implemented".to_string(),
            },
            ServerMsg::Save {} => ClientMsg::Error {
                status: "error".to_string(),
                message: "Not yet implemented".to_string(),
            },
        };
    }
}

// ─── Fonctions utilitaires (fournies) ───────────────────────────────────────
type WsStream = tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<std::net::TcpStream>>;

/// Lit un message du serveur et le désérialise.
fn read_server_msg(ws: &mut WsStream) -> Result<Option<ServerMsg>, Box<dyn Error>> {
    match ws.read()? {
        Message::Text(text) => Ok(serde_json::from_str(&text).ok()),
        _ => Ok(None),
    }
}

/// Sérialise et envoie un message au serveur.
fn send_client_msg(ws: &mut WsStream, msg: &ClientMsg) {
    let json = serde_json::to_string(msg).expect("sérialisation échouée");
    ws.send(Message::Text(json.into()))
        .expect("envoi WS échoué");
}
