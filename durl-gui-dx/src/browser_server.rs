//! Local HTTP server that listens for download requests from the browser extension.
//!
//! Endpoints:
//!   GET  /ping      — Returns 200 "pong"; extension uses this to detect if app is running.
//!   POST /download  — Accepts JSON `{url, cookies, filename?}` and forwards to the GUI via mpsc channel.
//!   OPTIONS *       — CORS pre-flight (browser extension may send one for the POST).

use ntex::http::Method;
use ntex::web::{self, App, HttpResponse};
use serde::{Deserialize, Serialize};
use std::sync::mpsc;

/// Port the server listens on. Must match the browser extension's APP_PORT constant.
pub const PORT: u16 = 19283;

/// Payload sent from the browser extension to the GUI.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct BrowserDownloadReq {
    pub url: String,
    #[serde(default)]
    pub cookies: String,
    #[serde(default)]
    pub filename: Option<String>,
}

/// Start the HTTP server in a background OS thread.
/// `tx` is used to send incoming download requests to the GUI event loop.
pub fn start_browser_server(tx: mpsc::Sender<BrowserDownloadReq>) {
    std::thread::Builder::new()
        .name("browser-server".into())
        .spawn(move || {
            if let Err(e) = ntex::rt::System::build()
                .build(ntex::rt::DefaultRuntime)
                .block_on(run_server(tx))
            {
                log::error!("[browser_server] server exited with error: {e}");
            }
        })
        .expect("failed to spawn browser-server thread");
}

async fn run_server(tx: mpsc::Sender<BrowserDownloadReq>) -> std::io::Result<()> {
    let shared = std::sync::Arc::new(tx);

    web::HttpServer::new(async move || {
        App::new()
            .state(shared.clone())
            // CORS pre-flight
            .route("/download", web::method(Method::OPTIONS).to(cors_preflight))
            .route("/ping", web::get().to(ping))
            .route("/download", web::post().to(download))
    })
    .bind(("127.0.0.1", PORT))?
    .run()
    .await
}

// ── Handlers ─────────────────────────────────────────────────────────────────

async fn ping() -> HttpResponse {
    HttpResponse::Ok()
        .header("Access-Control-Allow-Origin", "*")
        .body("pong")
}

async fn download(
    body: web::types::Json<BrowserDownloadReq>,
    state: web::types::State<std::sync::Arc<mpsc::Sender<BrowserDownloadReq>>>,
) -> HttpResponse {
    let req = body.into_inner();
    log::debug!("[browser_server] download request: {}", req.url);
    let _ = state.send(req);
    cors_ok(r#"{"status":"accepted"}"#)
}

async fn cors_preflight() -> HttpResponse {
    HttpResponse::NoContent()
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "POST, GET, OPTIONS")
        .header("Access-Control-Allow-Headers", "Content-Type")
        .header("Access-Control-Max-Age", "86400")
        .finish()
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn cors_ok(body: &'static str) -> HttpResponse {
    HttpResponse::Ok()
        .header("Access-Control-Allow-Origin", "*")
        .header("Content-Type", "application/json")
        .body(body)
}
