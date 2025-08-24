//! Web server implementation using Axum

use anyhow::Result;
use axum::{
    http::Method,
    response::Html,
    routing::{get, post},
    Router,
};
use log::{error, info};
use std::sync::Arc;
use tokio::sync::broadcast;
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
};

use super::{
    auth,
    database::Database,
    enhanced_websocket::{self, EnhancedWebSocketState},
    handlers, mock_sniper, websocket, WebConfig, WebSocketMessage,
};
use crate::SharedState;

/// Shared application state for the web server
#[derive(Clone)]
pub struct AppState {
    pub bot_state: SharedState,
    pub database: Arc<Database>,
    pub websocket_tx: broadcast::Sender<WebSocketMessage>,
    pub config: WebConfig,
    pub bot_start_time: std::time::Instant,
    pub bot_running: Arc<tokio::sync::RwLock<bool>>,
    pub discord: Option<crate::discord::DiscordAlert>,
    // Runtime bot configuration exposed via API
    pub runtime_config: Arc<tokio::sync::RwLock<super::BotConfig>>,
}

/// Web server for the dashboard
pub struct WebServer {
    app_state: AppState,
    enhanced_ws_state: EnhancedWebSocketState,
}

impl WebServer {
    /// Create a new web server instance
    pub async fn new(
        bot_state: SharedState,
        config: WebConfig,
        discord: Option<crate::discord::DiscordAlert>,
        main_config: &crate::Config,
    ) -> Result<Self> {
        // Initialize database
        let database = Arc::new(Database::new(&config.database_path).await?);

        // Create WebSocket broadcast channel
        let (websocket_tx, _) = broadcast::channel(1000);

        // Initialize runtime config from main config limits
        let runtime_config = super::BotConfig {
            min_profit_usd: main_config.limits.min_profit_usd,
            max_position_sol: main_config.limits.max_position_sol,
            max_daily_trades: main_config.limits.max_daily_trades,
            max_daily_loss_usd: main_config.limits.max_daily_loss_usd,
            enabled: true,
        };

        let app_state = AppState {
            bot_state,
            database,
            websocket_tx,
            config,
            bot_start_time: std::time::Instant::now(),
            bot_running: Arc::new(tokio::sync::RwLock::new(true)),
            discord,
            runtime_config: Arc::new(tokio::sync::RwLock::new(runtime_config)),
        };

        // Create enhanced WebSocket state separately
        let enhanced_ws_state = EnhancedWebSocketState::new(app_state.clone());

        Ok(Self {
            app_state,
            enhanced_ws_state,
        })
    }

    /// Start the web server
    pub async fn start(self) -> Result<()> {
        let config = self.app_state.config.clone();

        if !config.enabled {
            info!("🌐 Web dashboard is disabled");
            return Ok(());
        }

        info!(
            "🌐 Starting web dashboard on {}:{}",
            config.host, config.port
        );

        // Build CORS layer - allow frontend origin specifically
        let cors = CorsLayer::new()
            .allow_origin([
                "http://localhost:3000".parse().unwrap(),
                "http://127.0.0.1:3000".parse().unwrap(),
                "http://localhost:3001".parse().unwrap(),
                "http://127.0.0.1:3001".parse().unwrap(),
            ])
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::DELETE,
                Method::OPTIONS,
            ])
            .allow_headers([
                "content-type".parse().unwrap(),
                "authorization".parse().unwrap(),
                "x-requested-with".parse().unwrap(),
            ])
            .allow_credentials(true);

        // Build the router
        let app = Router::new()
            // API routes
            .route("/api/status", get(handlers::get_bot_status))
            .route("/api/config", get(handlers::get_bot_config))
            .route("/api/config", post(handlers::update_bot_config))
            .route("/api/transactions", get(handlers::get_transactions))
            .route("/api/stats", get(handlers::get_daily_stats))
            .route("/api/control/start", post(handlers::start_bot))
            .route("/api/control/stop", post(handlers::stop_bot))
            .route("/api/control/pause", post(handlers::pause_bot))
            .route("/api/control/emergency", post(handlers::emergency_stop))
            // WebSocket endpoint
            .route("/ws", get(websocket::websocket_handler))
            // Static files and dashboard
            .nest_service("/", ServeDir::new("dashboard-frontend/out"))
            // Add middleware
            .layer(
                ServiceBuilder::new()
                    .layer(cors)
                    .layer(auth::auth_middleware(config.auth_token.clone())),
            )
            .with_state(self.app_state.clone());

        // Create enhanced WebSocket router with separate state
        let enhanced_ws_router = Router::new()
            .route(
                "/enhanced-ws",
                get(enhanced_websocket::enhanced_websocket_handler),
            )
            .with_state(self.enhanced_ws_state.clone());

        // Merge routers
        let app = app.merge(enhanced_ws_router);

        // Create listener
        let listener =
            tokio::net::TcpListener::bind(format!("{}:{}", config.host, config.port)).await?;

        info!(
            "✅ Web dashboard started at http://{}:{}",
            config.host, config.port
        );
        info!("📊 Dashboard URL: http://{}:{}/", config.host, config.port);
        info!(
            "🔗 Enhanced WebSocket URL: ws://{}:{}/enhanced-ws",
            config.host, config.port
        );

        // Start mock sniper for testing
        mock_sniper::start_mock_sniper(self.enhanced_ws_state.clone()).await;
        info!("🎯 Mock sniper started for testing transaction flow");

        // Start the server
        axum::serve(listener, app).await?;

        Ok(())
    }

    /// Get the WebSocket sender for broadcasting messages
    pub fn get_websocket_sender(&self) -> broadcast::Sender<WebSocketMessage> {
        self.app_state.websocket_tx.clone()
    }

    /// Get the database reference
    pub fn get_database(&self) -> Arc<Database> {
        self.app_state.database.clone()
    }

    /// Get a clone of the runtime config handle
    pub fn get_runtime_config(&self) -> Arc<tokio::sync::RwLock<super::BotConfig>> {
        self.app_state.runtime_config.clone()
    }

    /// Get the enhanced WebSocket state for transaction handling
    pub fn get_enhanced_ws_state(&self) -> EnhancedWebSocketState {
        self.enhanced_ws_state.clone()
    }
}

/// Broadcast a WebSocket message to all connected clients
pub async fn broadcast_websocket_message(
    sender: &broadcast::Sender<WebSocketMessage>,
    message: WebSocketMessage,
) {
    if let Err(e) = sender.send(message) {
        error!("Failed to broadcast WebSocket message: {}", e);
    }
}
