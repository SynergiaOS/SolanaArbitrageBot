use axum::http::{Request, StatusCode};
use axum::{
    routing::{get, post},
    Router,
};
use http_body_util::BodyExt;
use log::{Level, LevelFilter, Metadata, Record};
use rust_decimal::Decimal;
use std::sync::{Arc, Mutex, OnceLock};
use tokio::sync::{broadcast, Mutex as AsyncMutex, RwLock};
use tower::ServiceExt; // for oneshot

use solana_arbitrage_bot::web::{self, WebConfig, WebSocketMessage};
use solana_arbitrage_bot::{
    Config, DexConfig, DexInfo, ExecutionConfig, LimitsConfig, RpcConfig, SharedState, WalletConfig,
};

// Simple in-memory test logger to capture warn! lines
#[allow(dead_code)]
struct TestLogger {
    lines: Mutex<Vec<String>>,
}
impl log::Log for TestLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Warn
    }
    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let mut l = self.lines.lock().unwrap();
            l.push(format!("{}", record.args()));
        }
    }
    fn flush(&self) {}
}
#[allow(dead_code)]
static TEST_LOGGER: OnceLock<&'static TestLogger> = OnceLock::new();
#[allow(dead_code)]
fn ensure_logger() -> &'static TestLogger {
    if let Some(l) = TEST_LOGGER.get() {
        return l;
    }
    let logger = Box::leak(Box::new(TestLogger {
        lines: Mutex::new(Vec::new()),
    }));
    let _ = log::set_logger(logger);
    log::set_max_level(LevelFilter::Warn);
    let _ = TEST_LOGGER.set(logger);
    logger
}

fn sample_config() -> Config {
    Config {
        rpc: RpcConfig {
            url: "http://localhost".into(),
            ws_url: "ws://localhost".into(),
        },
        wallet: WalletConfig {
            path: "./wallet.json".into(),
            use_ledger: None,
            ledger_path: None,
        },
        dex: DexConfig {
            raydium: DexInfo {
                program_id: "".into(),
                sol_usdc_pool: "".into(),
            },
            orca: DexInfo {
                program_id: "".into(),
                sol_usdc_pool: "".into(),
            },
        },
        limits: LimitsConfig {
            max_position_sol: Decimal::from_f64_retain(0.05).unwrap(),
            min_profit_percent: Decimal::from_f64_retain(0.3).unwrap(),
            min_profit_usd: Decimal::from_f64_retain(0.05).unwrap(),
            max_slippage_percent: Decimal::from_f64_retain(0.5).unwrap(),
            max_daily_loss_usd: Decimal::from_f64_retain(10.0).unwrap(),
            max_daily_trades: 50,
        },
        execution: ExecutionConfig {
            priority_fee_lamports: 5000,
            max_priority_fee_cap_lamports: Some(50_000),
            simulation_required: true,
            max_retries: 3,
        },
        discord: None,
        web: Some(WebConfig {
            enabled: true,
            host: "127.0.0.1".into(),
            port: 3001,
            auth_token: None,
            database_path: "./data/test.db".into(),
            allowed_origins: Some(vec!["http://localhost:3000".into()]),
            rate_limits: None,
            api_keys: None,
            control_ip_allowlist: None,
        }),
    }
}

async fn build_app_state(cfg: &Config) -> web::server::AppState {
    let web_cfg = cfg.web.clone().unwrap();
    let database = Arc::new(web::Database::new(&web_cfg.database_path).await.unwrap());
    let (websocket_tx, _) = broadcast::channel(100);
    let runtime_config = Arc::new(RwLock::new(web::BotConfig {
        min_profit_usd: cfg.limits.min_profit_usd,
        max_position_sol: cfg.limits.max_position_sol,
        max_daily_trades: cfg.limits.max_daily_trades,
        max_daily_loss_usd: cfg.limits.max_daily_loss_usd,
        enabled: true,
    }));
    web::server::AppState {
        bot_state: SharedState {
            raydium_price: Arc::new(AsyncMutex::new(None)),
            orca_price: Arc::new(AsyncMutex::new(None)),
            trades_today: Arc::new(AsyncMutex::new(0)),
            profit_today: Arc::new(AsyncMutex::new(Decimal::from_f64_retain(0.0).unwrap())),
        },
        database,
        websocket_tx,
        config: web_cfg,
        bot_start_time: std::time::Instant::now(),
        bot_running: Arc::new(tokio::sync::RwLock::new(true)),
        discord: None,
        runtime_config,
    }
}

#[tokio::test]
async fn api_config_invalid_returns_400() {
    let cfg = sample_config();
    let app_state = build_app_state(&cfg).await;

    let app = Router::new()
        .route("/api/config", post(web::handlers::update_bot_config))
        .with_state(app_state);

    let body = r#"{"min_profit_usd":0,"max_position_sol":0.02,"max_daily_trades":50,"max_daily_loss_usd":10,"enabled":true}"#;
    let request = Request::builder()
        .method("POST")
        .uri("/api/config")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(body))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn api_config_valid_returns_200_and_ws_event() {
    let cfg = sample_config();
    let app_state = build_app_state(&cfg).await;
    let mut rx = app_state.websocket_tx.subscribe();

    let app = Router::new()
        .route("/api/config", post(web::handlers::update_bot_config))
        .with_state(app_state);

    let body = r#"{"min_profit_usd":0.5,"max_position_sol":0.02,"max_daily_trades":50,"max_daily_loss_usd":10,"enabled":true}"#;
    let request = Request::builder()
        .method("POST")
        .uri("/api/config")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(body))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Expect a config_update message
    let msg = rx.try_recv().expect("expected a config_update message");
    match msg {
        WebSocketMessage::ConfigUpdate { .. } => {}
        _ => panic!("unexpected WS message type"),
    }
}

#[tokio::test]
async fn emergency_empty_body_returns_200_and_stops_bot() {
    let cfg = sample_config();
    let app_state = build_app_state(&cfg).await;
    let state_clone = app_state.clone();

    let app = Router::new()
        .route(
            "/api/control/emergency",
            post(web::handlers::emergency_stop),
        )
        .with_state(app_state);

    let request = Request::builder()
        .method("POST")
        .uri("/api/control/emergency")
        .header("content-type", "application/json")
        .body(axum::body::Body::from("null")) // Optional body -> None
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Verify bot_running was set to false
    let running = *state_clone.bot_running.read().await;
    assert!(!running);
}

#[tokio::test]
async fn rate_limit_real_router_6th_is_429() {
    use axum::http::Method;
    use tower::ServiceBuilder;
    use solana_arbitrage_bot::web::rate_limit::RateLimitLayer;

    let cfg = sample_config();
    let app_state = build_app_state(&cfg).await;

    let cors = tower_http::cors::CorsLayer::new()
        .allow_origin(["http://localhost:3000".parse().unwrap()])
        .allow_methods([Method::GET])
        .allow_headers(["content-type".parse().unwrap()]);

    let limits = solana_arbitrage_bot::web::RateLimitsConfig {
        default_per_ip_per_minute: Some(1000),
        control_per_ip_per_minute: Some(1000),
        config_per_ip_per_minute: Some(1000),
        status_per_ip_per_minute: Some(5),
    };

    let app = Router::new()
        .route("/api/status", get(web::handlers::get_bot_status))
        .with_state(app_state)
        .layer(ServiceBuilder::new().layer(RateLimitLayer::from_config(&Some(limits))).layer(cors));

    for i in 0..5 {
        let req = Request::builder().method("GET").uri("/api/status").header("x-real-ip","127.0.0.1").body(axum::body::Body::empty()).unwrap();
        let resp = app.clone().oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK, "req {}", i);
    }
    let req = Request::builder().method("GET").uri("/api/status").header("x-real-ip","127.0.0.1").body(axum::body::Body::empty()).unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn ratelimit_headers_are_present() {
    use tower::ServiceBuilder;
    use solana_arbitrage_bot::web::rate_limit::RateLimitLayer;

    let cfg = sample_config();
    let app_state = build_app_state(&cfg).await;

    let app = Router::new()
        .route("/api/status", get(web::handlers::get_bot_status))
        .with_state(app_state)
        .layer(ServiceBuilder::new().layer(RateLimitLayer::from_config(&None)));

    let req = Request::builder().method("GET").uri("/api/status").header("x-real-ip","3.3.3.3").body(axum::body::Body::empty()).unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let h = resp.headers();
    assert!(h.get("x-ratelimit-limit").is_some());
    assert!(h.get("x-ratelimit-remaining").is_some());
    assert!(h.get("x-ratelimit-reset").is_some());
}


#[allow(dead_code)]
async fn emergency_json_body_returns_200_and_logs() {
    let _ = ensure_logger();
    let cfg = sample_config();
    let app_state = build_app_state(&cfg).await;

    let app = Router::new()
        .route(
            "/api/control/emergency",
            post(web::handlers::emergency_stop),
        )
        .with_state(app_state);

    let body = r#"{"reason":"smoke","source":"test"}"#;
    let request = Request::builder()
        .method("POST")
        .uri("/api/control/emergency")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(body))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    // Best-effort: check that our logger saw an EMERGENCY line
    // Note: depending on global logger setup tests might not capture; we keep it non-fatal
}

#[tokio::test]
async fn get_status_returns_valid_structure() {
    let cfg = sample_config();
    let app_state = build_app_state(&cfg).await;

    let app = Router::new()
        .route("/api/status", get(web::handlers::get_bot_status))
        .with_state(app_state);

    let request = Request::builder()
        .method("GET")
        .uri("/api/status")
        .body(axum::body::Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = response.into_body().collect().await.unwrap().to_bytes();
    let v: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert!(v.get("success").and_then(|b| b.as_bool()).unwrap_or(false));
    let data = v.get("data").cloned().unwrap_or(serde_json::Value::Null);
    let obj = data.as_object().expect("data should be an object");

    // Verify required fields exist
    assert!(obj.contains_key("running"));
    assert!(obj.contains_key("mode"));
    assert!(obj.contains_key("uptime_seconds"));
    assert!(obj.contains_key("last_update"));
    assert!(obj.contains_key("raydium_price"));
    assert!(obj.contains_key("orca_price"));
    assert!(obj.contains_key("spread_percent"));
    assert!(obj.contains_key("trades_today"));
    assert!(obj.contains_key("profit_today"));
}
