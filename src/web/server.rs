//! Web server implementation using Axum

use crate::web::metrics::{metrics_handler, MetricsLayer};
use crate::web::rate_limit::RateLimitLayer;
use anyhow::Result;
use axum::{
    http::Method,
    routing::{get, post},
    Router,
};
use log::{error, info};
use std::sync::Arc;
use tokio::sync::broadcast;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, services::ServeDir, timeout::TimeoutLayer};

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

        // Build secure CORS layer - restrict to specific origins
        let allowed = self
            .app_state
            .config
            .allowed_origins
            .clone()
            .unwrap_or_else(|| {
                vec![
                    "http://localhost:3000".to_string(),
                    "http://127.0.0.1:3000".to_string(),
                ]
            });
        let cors = CorsLayer::new()
            .allow_origin(
                allowed
                    .into_iter()
                    .filter_map(|o| o.parse().ok())
                    .collect::<Vec<_>>(),
            )
            .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
            .allow_headers([
                "content-type".parse().unwrap(),
                "authorization".parse().unwrap(),
                "x-api-key".parse().unwrap(),
            ])
            .allow_credentials(false); // Disable credentials for security

        // Build the router
        let app = Router::new()
            // API routes
            .route("/api/status", get(handlers::get_bot_status))
            .route("/api/config", get(handlers::get_bot_config))
            .route("/api/config", post(handlers::update_bot_config))
            .route("/api/transactions", get(handlers::get_transactions))
            .route("/api/stats", get(handlers::get_daily_stats))
            // Prometheus metrics endpoint (public)
            .route("/metrics", get(metrics_handler))
            // WebSocket endpoint
            .route("/ws", get(websocket::websocket_handler))
            // Static files and dashboard
            .nest_service("/", ServeDir::new("dashboard-frontend/out"))
            // Add security middleware
            .layer(
                ServiceBuilder::new()
                    .layer(TimeoutLayer::new(std::time::Duration::from_secs(30))) // Request timeout
                    .layer(MetricsLayer)
                    .layer(RateLimitLayer::from_config(
                        &self.app_state.config.rate_limits,
                    ))
                    .layer(cors)
                    .layer(auth::auth_middleware_from_config(&self.app_state.config)),
            )
            .with_state(self.app_state.clone());

        // Per-route hardened routers
        let control_router = Router::new()
            .route("/api/control/start", post(handlers::start_bot))
            .route("/api/control/stop", post(handlers::stop_bot))
            .route("/api/control/pause", post(handlers::pause_bot))
            .route("/api/control/emergency", post(handlers::emergency_stop))
            .route_layer(tower_http::limit::RequestBodyLimitLayer::new(64 * 1024))
            .route_layer(
                ServiceBuilder::new()
                    .layer(axum::error_handling::HandleErrorLayer::new(
                        |_err: tower::BoxError| async move {
                            (
                                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                                "service overloaded",
                            )
                        },
                    ))
                    .layer(tower::limit::ConcurrencyLimitLayer::new(128))
                    .layer(tower::buffer::BufferLayer::new(1024)),
            )
            .route_layer(
                tower_http::set_header::SetResponseHeaderLayer::if_not_present(
                    axum::http::header::STRICT_TRANSPORT_SECURITY,
                    axum::http::HeaderValue::from_static(
                        "max-age=31536000; includeSubDomains; preload",
                    ),
                ),
            )
            .route_layer(
                tower_http::set_header::SetResponseHeaderLayer::if_not_present(
                    axum::http::HeaderName::from_static("x-content-type-options"),
                    axum::http::HeaderValue::from_static("nosniff"),
                ),
            )
            .route_layer(
                tower_http::set_header::SetResponseHeaderLayer::if_not_present(
                    axum::http::HeaderName::from_static("referrer-policy"),
                    axum::http::HeaderValue::from_static("no-referrer"),
                ),
            )
            .with_state(self.app_state.clone());

        let config_router = Router::new()
            .route("/api/config", post(handlers::update_bot_config))
            .route_layer(tower_http::limit::RequestBodyLimitLayer::new(64 * 1024))
            .route_layer(
                ServiceBuilder::new()
                    .layer(axum::error_handling::HandleErrorLayer::new(
                        |_err: tower::BoxError| async move {
                            (
                                axum::http::StatusCode::SERVICE_UNAVAILABLE,
                                "service overloaded",
                            )
                        },
                    ))
                    .layer(tower::limit::ConcurrencyLimitLayer::new(128))
                    .layer(tower::buffer::BufferLayer::new(1024)),
            )
            .route_layer(
                tower_http::set_header::SetResponseHeaderLayer::if_not_present(
                    axum::http::header::STRICT_TRANSPORT_SECURITY,
                    axum::http::HeaderValue::from_static(
                        "max-age=31536000; includeSubDomains; preload",
                    ),
                ),
            )
            .route_layer(
                tower_http::set_header::SetResponseHeaderLayer::if_not_present(
                    axum::http::HeaderName::from_static("x-content-type-options"),
                    axum::http::HeaderValue::from_static("nosniff"),
                ),
            )
            .route_layer(
                tower_http::set_header::SetResponseHeaderLayer::if_not_present(
                    axum::http::HeaderName::from_static("referrer-policy"),
                    axum::http::HeaderValue::from_static("no-referrer"),
                ),
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
        let app = app
            .merge(control_router)
            .merge(config_router)
            .merge(enhanced_ws_router);

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
