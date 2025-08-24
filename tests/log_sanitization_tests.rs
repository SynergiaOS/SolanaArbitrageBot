use std::sync::Mutex;

use axum::http::{Request, StatusCode};
use axum::{routing::get, Router};

use log::{Level, LevelFilter, Metadata, Record};

use tower::ServiceExt;

// Test logger that captures logs in memory
struct TestLogger {
    lines: Mutex<Vec<String>>,
}
impl log::Log for TestLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }
    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let mut l = self.lines.lock().unwrap();
            l.push(format!("{}", record.args()));
        }
    }
    fn flush(&self) {}
}

fn init_logger() -> &'static TestLogger {
    static LOGGER: std::sync::OnceLock<&'static TestLogger> = std::sync::OnceLock::new();
    if let Some(l) = LOGGER.get() {
        return l;
    }
    let logger = Box::leak(Box::new(TestLogger {
        lines: Mutex::new(Vec::new()),
    }));
    let _ = log::set_logger(logger);
    log::set_max_level(LevelFilter::Info);
    let _ = LOGGER.set(logger);
    logger
}

#[tokio::test]
async fn no_sensitive_keywords_in_logs_for_basic_requests() {
    let logger = init_logger();

    // Build a trivial app with one handler that logs a message
    let app = Router::new().route(
        "/echo",
        get(|| async {
            log::info!("Echo handler called");
            "ok"
        }),
    );

    // Perform a request that should not log secrets
    let req = Request::builder()
        .method("GET")
        .uri("/echo")
        .body(axum::body::Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Scan captured logs for sensitive keywords
    let forbidden = [
        "seed",
        "mnemonic",
        "secret",
        "private_key",
        "private key",
        "master key",
    ];

    let lines = logger.lines.lock().unwrap();
    let all_logs = lines.join("\n").to_lowercase();
    for kw in &forbidden {
        assert!(
            !all_logs.contains(kw),
            "found forbidden keyword '{kw}' in logs: {all_logs}"
        );
    }
}
