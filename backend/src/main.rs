mod models;
mod db;
mod routes;
mod margin_calculator;
mod position_monitor;
mod pnl_tracker;
mod position_manager;
mod utils;

use axum::{
    http::Method,
    Router,
};
use tower_http::cors::{Any, CorsLayer};
use std::{env, sync::Arc};
use log::info;

use margin_calculator::MarginCalculator;
use position_monitor::PositionMonitor;
use pnl_tracker::PnlTracker;

#[tokio::main]
async fn main() {
    env_logger::init();
    dotenv::dotenv().ok();

    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://postgres:password@localhost/position_management".to_string());

    // Create database connection pool
    let pool = db::create_pool(&database_url).await
        .expect("Failed to create database pool");

    // Initialize services
    let margin_calculator = Arc::new(MarginCalculator::new());
    let position_monitor = Arc::new(PositionMonitor::new(pool.clone()));
    let pnl_tracker = Arc::new(PnlTracker::new(pool.clone()));

    // Start position monitoring in background
    let monitor = position_monitor.clone();
    tokio::spawn(async move {
        monitor.start_monitoring().await;
    });
    info!("Started position monitoring service");

    // Create CORS layer
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers(Any);

    // Check if Solana integration is enabled
    let solana_enabled = std::env::var("PROGRAM_ID").is_ok() && std::env::var("WALLET_PATH").is_ok();
    if solana_enabled {
        info!("Solana integration enabled - Program ID and Wallet configured");
    } else {
        info!("Solana integration disabled - Missing PROGRAM_ID or WALLET_PATH env vars");
    }

    // Create application state
    let app_state = routes::AppState {
        pool: pool.clone(),
        margin_calculator,
        position_monitor,
        pnl_tracker,
        solana_enabled,
    };

    // Create the main application with all routes
    let app = routes::create_routes(app_state)
        .layer(cors)
        .layer(tower::ServiceBuilder::new()
            .layer(tower_http::trace::TraceLayer::new_for_http()));

    let host = env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = env::var("SERVER_PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("{}:{}", host, port);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    info!("Position Management API server running on http://{}", addr);
    
    axum::serve(listener, app).await.unwrap();
}
