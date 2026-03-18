use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use clap::{Parser, Subcommand};
use tokio::sync::{broadcast, RwLock};
use tracing_subscriber::EnvFilter;

mod api;
mod app_state;
mod middleware;
mod ws;

use app_state::AppState;

#[derive(Parser)]
#[command(name = "bilbycast-manager", about = "Bilbycast Edge Node Manager")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initial setup: create database and first super_admin user
    Setup {
        /// Path to configuration file
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// Start the manager server
    Serve {
        /// Path to configuration file
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
        /// Override listen port
        #[arg(short, long)]
        port: Option<u16>,
    },
    /// Reset a user's password
    ResetPassword {
        /// Username to reset
        #[arg(long)]
        username: String,
        /// Path to configuration file
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// Export all data to JSON
    Export {
        /// Output file path
        #[arg(short, long)]
        output: String,
        /// Path to configuration file
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
    /// Import data from JSON
    Import {
        /// Input file path
        #[arg(short, long)]
        input: String,
        /// Path to configuration file
        #[arg(short, long, default_value = "config/default.toml")]
        config: String,
    },
}

fn main() -> anyhow::Result<()> {
    // Load .env BEFORE starting tokio runtime (set_var is safe here: single-threaded)
    load_dotenv();

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async_main())
}

async fn async_main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Setup { config } => run_setup(&config).await,
        Commands::Serve { config, port } => run_serve(&config, port).await,
        Commands::ResetPassword { username, config } => {
            run_reset_password(&config, &username).await
        }
        Commands::Export { output, config } => run_export(&config, &output).await,
        Commands::Import { input, config } => run_import(&config, &input).await,
    }
}

/// Load .env file from current directory or parent directories.
fn load_dotenv() {
    // Check CWD first, then parent dirs
    for name in &[".env", "../.env"] {
        let path = std::path::Path::new(name);
        if path.exists() {
            if let Ok(contents) = std::fs::read_to_string(path) {
                for line in contents.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with('#') {
                        continue;
                    }
                    if let Some((key, value)) = line.split_once('=') {
                        let key = key.trim();
                        let value = value.trim();
                        // Don't override existing env vars
                        if std::env::var(key).is_err() {
                            // SAFETY: Called before tokio runtime starts (single-threaded).
                            unsafe { std::env::set_var(key, value) };
                        }
                    }
                }
            }
            break;
        }
    }
}

async fn run_setup(config_path: &str) -> anyhow::Result<()> {
    let server_config = load_config(config_path)?;
    let pool = manager_core::db::init_db(&server_config.database_url).await?;

    let count = manager_core::db::users::count_users(&pool).await?;
    if count > 0 {
        println!(
            "Database already has {} user(s). Setup is only for first-time initialization.",
            count
        );
        return Ok(());
    }

    println!("=== bilbycast-manager Setup ===\n");

    print!("Super admin username: ");
    let username = read_line()?;

    print!("Display name: ");
    let display_name = read_line()?;

    print!("Email (optional): ");
    let email_input = read_line()?;
    let email = if email_input.is_empty() {
        None
    } else {
        Some(email_input)
    };

    let password = rpassword_read("Password: ")?;
    let password_confirm = rpassword_read("Confirm password: ")?;

    if password != password_confirm {
        anyhow::bail!("Passwords do not match");
    }

    manager_core::auth::validate_password(&password).map_err(|e| anyhow::anyhow!(e))?;

    let req = manager_core::models::CreateUserRequest {
        username,
        password,
        display_name,
        email,
        role: manager_core::models::UserRole::SuperAdmin,
        is_temporary: false,
        expires_at: None,
        allowed_node_ids: None,
    };

    let user = manager_core::db::users::create_user(&pool, &req).await?;
    println!(
        "\nSuper admin user '{}' created successfully (ID: {}).",
        user.username, user.id
    );
    println!("You can now start the server with: bilbycast-manager serve");

    Ok(())
}

async fn run_serve(config_path: &str, port_override: Option<u16>) -> anyhow::Result<()> {
    let server_config = load_config(config_path)?;
    let pool = manager_core::db::init_db(&server_config.database_url).await?;

    let port = port_override.unwrap_or(server_config.listen_port);

    // Check if setup has been run
    let count = manager_core::db::users::count_users(&pool).await?;
    if count == 0 {
        tracing::warn!(
            "No users found. Run 'bilbycast-manager setup' to create the first admin user."
        );
    }

    // Load secrets from environment variables (REQUIRED)
    let jwt_secret = load_required_env_secret("BILBYCAST_JWT_SECRET")?;
    let master_key_str = load_required_env_secret("BILBYCAST_MASTER_KEY")?;
    let master_key = manager_core::crypto::derive_key(&master_key_str);

    let (browser_tx, _) = broadcast::channel(256);

    // Create node auth rate limiter: 5 failed attempts per 60 seconds = lockout
    let node_auth_limiter = Arc::new(ws::node_hub::NodeAuthLimiter::new(5, 60));

    let node_hub = Arc::new(ws::node_hub::NodeHub::new(
        pool.clone(),
        browser_tx.clone(),
        node_auth_limiter.clone(),
    ));

    let state = AppState {
        db: pool,
        node_hub,
        jwt_secret: jwt_secret.as_bytes().to_vec(),
        master_key,
        browser_stats_tx: browser_tx,
        config: Arc::new(RwLock::new(server_config.clone())),
    };

    let app = build_router(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    // Check for TLS configuration
    let tls_cert = std::env::var("BILBYCAST_TLS_CERT")
        .ok()
        .or_else(|| server_config.tls.as_ref().map(|t| t.cert_path.clone()));
    let tls_key = std::env::var("BILBYCAST_TLS_KEY")
        .ok()
        .or_else(|| server_config.tls.as_ref().map(|t| t.key_path.clone()));

    match (tls_cert, tls_key) {
        #[cfg(feature = "tls")]
        (Some(cert_path), Some(key_path)) => {
            tracing::info!(
                "bilbycast-manager listening on {addr} with TLS (HTTPS/WSS)"
            );

            let tls_config = axum_server::tls_rustls::RustlsConfig::from_pem_file(
                &cert_path,
                &key_path,
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to load TLS cert/key: {e}"))?;

            axum_server::bind_rustls(addr, tls_config)
                .serve(app.into_make_service())
                .await?;
        }
        #[cfg(not(feature = "tls"))]
        (Some(_), Some(_)) => {
            anyhow::bail!(
                "TLS cert/key configured but the 'tls' feature is not enabled. \
                 Rebuild with: cargo build --features tls"
            );
        }
        (Some(_), None) | (None, Some(_)) => {
            anyhow::bail!("Both BILBYCAST_TLS_CERT and BILBYCAST_TLS_KEY must be set for TLS");
        }
        (None, None) => {
            tracing::warn!(
                "TLS not configured — running in plaintext HTTP/WS mode. \
                 Set BILBYCAST_TLS_CERT and BILBYCAST_TLS_KEY for production."
            );
            tracing::info!("bilbycast-manager listening on {addr}");

            let listener = tokio::net::TcpListener::bind(addr).await?;
            axum::serve(listener, app).await?;
        }
    }

    Ok(())
}

/// Load a required secret from an environment variable.
/// Refuses to start with weak/default values.
fn load_required_env_secret(name: &str) -> anyhow::Result<String> {
    let value = std::env::var(name).map_err(|_| {
        anyhow::anyhow!(
            "Environment variable {name} is not set.\n\
             Set it in your .env file or shell:\n\
             {name}=$(openssl rand -hex 32)"
        )
    })?;

    if value.is_empty() {
        anyhow::bail!("{name} is empty");
    }

    // Reject known-weak defaults that might be copy-pasted
    let weak_values = [
        "change-me",
        "default",
        "secret",
        "password",
        "bilbycast-manager-default",
        "bilbycast-default",
    ];
    let lower = value.to_lowercase();
    for weak in &weak_values {
        if lower.contains(weak) {
            anyhow::bail!(
                "{name} contains a weak/default value. Generate a proper secret:\n\
                 {name}=$(openssl rand -hex 32)"
            );
        }
    }

    if value.len() < 16 {
        anyhow::bail!(
            "{name} is too short ({}). Use at least 32 characters:\n\
             {name}=$(openssl rand -hex 32)",
            value.len()
        );
    }

    Ok(value)
}

async fn run_reset_password(config_path: &str, username: &str) -> anyhow::Result<()> {
    let server_config = load_config(config_path)?;
    let pool = manager_core::db::init_db(&server_config.database_url).await?;

    let user = manager_core::db::users::get_user_by_username(&pool, username)
        .await?
        .ok_or_else(|| anyhow::anyhow!("User '{}' not found", username))?;

    let password = rpassword_read("New password: ")?;
    let password_confirm = rpassword_read("Confirm password: ")?;

    if password != password_confirm {
        anyhow::bail!("Passwords do not match");
    }

    manager_core::auth::validate_password(&password).map_err(|e| anyhow::anyhow!(e))?;

    let req = manager_core::models::UpdateUserRequest {
        password: Some(password),
        display_name: None,
        email: None,
        role: None,
        is_temporary: None,
        expires_at: None,
        allowed_node_ids: None,
        is_active: None,
    };

    manager_core::db::users::update_user(&pool, &user.id, &req).await?;
    println!("Password for '{}' has been reset.", username);

    Ok(())
}

async fn run_export(config_path: &str, output_path: &str) -> anyhow::Result<()> {
    let server_config = load_config(config_path)?;
    let pool = manager_core::db::init_db(&server_config.database_url).await?;

    let export =
        manager_core::export::export_all(&pool, "cli-export", true, Some(30), true).await?;
    let json = serde_json::to_string_pretty(&export)?;
    std::fs::write(output_path, json)?;

    println!("Exported to {output_path}");
    Ok(())
}

async fn run_import(config_path: &str, input_path: &str) -> anyhow::Result<()> {
    let server_config = load_config(config_path)?;
    let _pool = manager_core::db::init_db(&server_config.database_url).await?;

    let _json = std::fs::read_to_string(input_path)?;
    // TODO: implement import logic
    println!("Import from {input_path} - not yet implemented");
    Ok(())
}

mod ui;

fn build_router(state: AppState) -> Router {
    use tower_http::cors::CorsLayer;
    use tower_http::trace::TraceLayer;

    let api_routes = api::build_api_router(state.clone());
    let ws_routes = ws::build_ws_router(state.clone());
    let ui_routes = ui::build_ui_router();

    Router::new()
        .merge(api_routes)
        .merge(ws_routes)
        .merge(ui_routes)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state)
}

/// TLS configuration section in TOML.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct TlsConfig {
    pub cert_path: String,
    pub key_path: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ServerConfig {
    #[serde(default = "default_listen_port")]
    pub listen_port: u16,
    #[serde(default = "default_database_url")]
    pub database_url: String,
    /// TLS certificate/key paths (optional, can also be set via env vars).
    pub tls: Option<TlsConfig>,
}

fn default_listen_port() -> u16 {
    8443
}

fn default_database_url() -> String {
    "sqlite:bilbycast-manager.db?mode=rwc".to_string()
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            listen_port: 8443,
            database_url: default_database_url(),
            tls: None,
        }
    }
}

fn load_config(path: &str) -> anyhow::Result<ServerConfig> {
    let mut config = if std::path::Path::new(path).exists() {
        let content = std::fs::read_to_string(path)?;
        toml::from_str(&content)?
    } else {
        tracing::warn!("Config file not found at {path}, using defaults");
        ServerConfig::default()
    };

    // Environment variable overrides for non-secret config
    if let Ok(port) = std::env::var("BILBYCAST_PORT") {
        if let Ok(p) = port.parse() {
            config.listen_port = p;
        }
    }
    if let Ok(db_url) = std::env::var("BILBYCAST_DATABASE_URL") {
        config.database_url = db_url;
    }

    Ok(config)
}

fn read_line() -> anyhow::Result<String> {
    use std::io::{self, Write};
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

fn rpassword_read(prompt: &str) -> anyhow::Result<String> {
    print!("{prompt}");
    read_line()
}
