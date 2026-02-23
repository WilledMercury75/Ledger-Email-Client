mod api;
mod crypto;
mod dht;
mod fallback;
mod gmail;
mod models;
mod p2p;
mod store;

use std::path::PathBuf;
use std::sync::Arc;

use actix_cors::Cors;
use actix_web::{web, App, HttpServer};
use clap::Parser;
use tokio::sync::mpsc;

use crypto::keys::LedgerIdentity;
use p2p::node::P2PCommand;
use store::db::Database;

/// Shared application state
pub struct AppState {
    pub identity: Arc<LedgerIdentity>,
    pub db: Arc<Database>,
    pub p2p_tx: mpsc::Sender<P2PCommand>,
    pub peer_id: libp2p::PeerId,
}

/// Ledger Core — Decentralized Encrypted Mail Engine
#[derive(Parser, Debug)]
#[command(name = "ledger-core", version, about)]
struct Args {
    /// REST API port
    #[arg(long, default_value_t = 8420)]
    port: u16,

    /// libp2p swarm port
    #[arg(long, default_value_t = 9420)]
    p2p_port: u16,

    /// Data directory
    #[arg(long)]
    data_dir: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();

    // Determine data directory
    let data_dir = if let Some(ref dir) = args.data_dir {
        PathBuf::from(dir)
    } else {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("ledger")
    };

    tracing::info!("Data directory: {:?}", data_dir);

    // Initialize identity
    let identity = Arc::new(LedgerIdentity::load_or_create(&data_dir)?);
    tracing::info!("Ledger ID: {}", identity.ledger_id);

    // Initialize database
    let db = Arc::new(Database::open(&data_dir)?);
    tracing::info!("Database initialized");

    // Start P2P node
    let (p2p_tx, peer_id) = p2p::node::start_node(
        args.p2p_port,
        identity.clone(),
        db.clone(),
    ).await?;

    tracing::info!("P2P node started, peer ID: {}", peer_id);

    // Start REST API server
    let api_port = args.port;
    let state = web::Data::new(AppState {
        identity: identity.clone(),
        db: db.clone(),
        p2p_tx,
        peer_id,
    });

    tracing::info!("Starting REST API on 127.0.0.1:{}", api_port);

    println!("\n╔══════════════════════════════════════════╗");
    println!("║         LEDGER CORE v0.1.0               ║");
    println!("╠══════════════════════════════════════════╣");
    println!("║  Ledger ID: {}...  ║", &identity.ledger_id[..30]);
    println!("║  API:       http://127.0.0.1:{:<5}       ║", api_port);
    println!("║  P2P:       /ip4/0.0.0.0/tcp/{:<5}      ║", args.p2p_port);
    println!("║  Peer ID:   {}... ║", &peer_id.to_string()[..30]);
    println!("╚══════════════════════════════════════════╝\n");

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .wrap(cors)
            .app_data(state.clone())
            // Identity
            .service(api::identity::get_identity)
            // Messages
            .service(api::messages::list_messages)
            .service(api::messages::get_message)
            .service(api::messages::send_message)
            .service(api::messages::delete_message)
            // Peers
            .service(api::peers::list_peers)
            .service(api::peers::connect_peer)
            // Gmail
            .service(api::gmail::get_gmail_config)
            .service(api::gmail::set_gmail_config)
            .service(api::gmail::fetch_gmail)
            .service(api::gmail::send_gmail)
            // Settings & Contacts
            .service(api::settings::get_settings)
            .service(api::settings::update_settings)
            .service(api::settings::list_contacts)
            .service(api::settings::add_contact)
    })
    .bind(format!("127.0.0.1:{}", api_port))?
    .run()
    .await?;

    Ok(())
}
