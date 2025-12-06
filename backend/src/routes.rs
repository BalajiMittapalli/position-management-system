use axum::{
    extract::{Path, Query, State, WebSocketUpgrade, ws::WebSocket, ws::Message},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
    routing::{get, post, put, delete},
    Router,
};
use serde_json::json;
use sqlx::PgPool;
use bigdecimal::{BigDecimal, FromPrimitive};
use std::{str::FromStr, sync::Arc};
use uuid::Uuid;
use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;

use crate::models::*;
use crate::margin_calculator::MarginCalculator;
use crate::position_monitor::PositionMonitor;
use crate::pnl_tracker::PnlTracker;
use crate::position_manager::PositionManager;

// Application state with all services
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub margin_calculator: Arc<MarginCalculator>,
    pub position_monitor: Arc<PositionMonitor>,
    pub pnl_tracker: Arc<PnlTracker>,
    pub solana_enabled: bool,
}

// Create the main router with all API routes
pub fn create_routes(state: AppState) -> Router {
    Router::new()
        .route("/api/positions/open", post(open_position))
        .route("/api/positions/:id", get(get_position))
        .route("/api/positions/:id/close", delete(close_position))
        .route("/api/positions/:id/modify", put(modify_position))
        .route("/api/ws", get(websocket_handler))
        .route("/api/users/:id/positions", get(get_user_positions))
        .route("/api/users/:id/stats", get(get_user_stats))
        .route("/api/funding-rates", get(get_funding_rates))
        .route("/api/margin/calculate", post(calculate_margin))
        .route("/api/positions/health-check", post(position_health_check))
        .route("/api/health", get(health_check))
        .route("/api/solana/status", get(solana_status))
        .with_state(state)
}

#[derive(Deserialize)]
pub struct GetPositionsQuery {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(Deserialize)]
pub struct ModifyPositionPayload {
    pub modification_type: String,  // "increase_size", "decrease_size", "add_margin", "remove_margin"
    pub amount: String,             // Amount to modify
    pub new_entry_price: Option<String>,  // For averaging when increasing size
}


// Basic health check endpoint
pub async fn health_check() -> impl IntoResponse {
    Json(json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

// Solana integration status check
pub async fn solana_status(State(state): State<AppState>) -> impl IntoResponse {
    Json(json!({
        "solana_enabled": state.solana_enabled,
        "status": if state.solana_enabled { "connected" } else { "disabled" },
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

// Open a new position
pub async fn open_position(
    State(state): State<AppState>,
    Json(request): Json<PositionRequest>,
) -> Result<impl IntoResponse, AppError> {
    // Validate input
    if request.size <= BigDecimal::from_i32(0).unwrap() {
        return Err(AppError::BadRequest("Position size must be positive".to_string()));
    }

    // Calculate required margin
    let margin = state.margin_calculator.calculate_required_margin(
        &request.entry_price,
        &request.size,
        request.leverage as u8,
    )?;

    // Create position in database
    let position_id = Uuid::new_v4().to_string();
    let side_str = match request.side {
        Side::Long => "Long",
        Side::Short => "Short",
    };

    sqlx::query(
        r#"
        INSERT INTO positions (
            id, user_id, symbol, side, size, entry_price, 
            margin, leverage, status, created_at, updated_at
        ) VALUES ($1, $2, $3, $4::side_type, $5, $6, $7, $8, 'open', $9, $10)
        "#
    )
    .bind(&position_id)
    .bind(&request.user_id)
    .bind(&request.symbol)
    .bind(side_str)
    .bind(&request.size)
    .bind(&request.entry_price)
    .bind(&margin)
    .bind(request.leverage as i16)
    .bind(Utc::now())
    .bind(Utc::now())
    .execute(&state.pool)
    .await
    .map_err(AppError::Database)?;

    // Create PnL snapshot
    let _snapshot_result = state.pnl_tracker.create_pnl_snapshot(
        &request.user_id,
        &BigDecimal::from_i32(0).unwrap(),
        &BigDecimal::from_i32(0).unwrap(),
        &BigDecimal::from_i32(0).unwrap(),
    ).await;

    let response_data = json!({
        "position_id": position_id,
        "user_id": request.user_id,
        "symbol": request.symbol,
        "side": side_str,
        "size": request.size,
        "entry_price": request.entry_price,
        "margin": margin,
        "leverage": request.leverage,
        "status": "open"
    });

    Ok(Json(ApiResponse::success(response_data)))
}

// Get position by ID
pub async fn get_position(
    State(state): State<AppState>,
    Path(position_id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let position = sqlx::query_as::<_, Position>(
        "SELECT * FROM positions WHERE id = $1"
    )
    .bind(&position_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(AppError::Database)?;

    match position {
        Some(pos) => {
            let response_data = json!({
                "id": pos.id,
                "user_id": pos.user_id,
                "symbol": pos.symbol,
                "side": pos.side,
                "size": pos.size,
                "entry_price": pos.entry_price,
                "margin": pos.margin,
                "leverage": pos.leverage,
                "status": pos.status,
                "unrealized_pnl": pos.unrealized_pnl,
                "realized_pnl": pos.realized_pnl,
                "created_at": pos.created_at,
                "updated_at": pos.updated_at
            });
            Ok(Json(ApiResponse::success(response_data)))
        }
        None => Err(AppError::NotFound("Position not found".to_string()))
    }
}

// Close position
pub async fn close_position(
    State(state): State<AppState>,
    Path(position_id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    // Get position
    let position = sqlx::query_as::<_, Position>(
        "SELECT * FROM positions WHERE id = $1 AND status = 'open'"
    )
    .bind(&position_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(AppError::Database)?;

    let position = position.ok_or_else(|| 
        AppError::NotFound("Open position not found".to_string())
    )?;

    // For now, just mark as closed without calculating exit price
    sqlx::query!(
        "UPDATE positions SET status = 'closed', updated_at = $2 WHERE id = $1",
        position_id,
        Utc::now(),
    )
    .execute(&state.pool)
    .await
    .map_err(AppError::Database)?;

    let response_data = json!({
        "position_id": position_id,
        "status": "closed",
        "message": "Position closed successfully"
    });

    Ok(Json(ApiResponse::success(response_data)))
}

// Get user positions
pub async fn get_user_positions(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
    Query(params): Query<GetPositionsQuery>,
) -> Result<impl IntoResponse, AppError> {
    let limit = params.limit.unwrap_or(50).min(100);
    let offset = params.offset.unwrap_or(0);

    let positions = sqlx::query_as::<_, Position>(
        "SELECT * FROM positions WHERE user_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"
    )
    .bind(&user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await
    .map_err(AppError::Database)?;

    let response_data = json!({
        "user_id": user_id,
        "positions": positions,
        "count": positions.len(),
        "limit": limit,
        "offset": offset
    });

    Ok(Json(ApiResponse::success(response_data)))
}

// Get user statistics
pub async fn get_user_stats(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let total_positions = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM positions WHERE user_id = $1"
    )
    .bind(&user_id)
    .fetch_one(&state.pool)
    .await
    .map_err(AppError::Database)?;

    let open_positions = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM positions WHERE user_id = $1 AND status = 'open'"
    )
    .bind(&user_id)
    .fetch_one(&state.pool)
    .await
    .map_err(AppError::Database)?;

    let response_data = json!({
        "user_id": user_id,
        "total_positions": total_positions,
        "open_positions": open_positions
    });

    Ok(Json(ApiResponse::success(response_data)))
}

// Get funding rates (placeholder)
pub async fn get_funding_rates() -> impl IntoResponse {
    let response_data = json!({
        "BTC/USD": 0.0001,
        "ETH/USD": 0.0001,
        "SOL/USD": 0.0001
    });

    Json(ApiResponse::success(response_data))
}

// Calculate margin requirements
pub async fn calculate_margin(
    State(state): State<AppState>,
    Json(request): Json<MarginRequest>,
) -> Result<impl IntoResponse, AppError> {
    let required_margin = state.margin_calculator.calculate_required_margin(
        &request.entry_price,
        &request.size,
        request.leverage,
    )?;

    let response_data = json!({
        "entry_price": request.entry_price,
        "size": request.size,
        "leverage": request.leverage,
        "required_margin": required_margin
    });

    Ok(Json(ApiResponse::success(response_data)))
}

// Position health check
pub async fn position_health_check(
    State(state): State<AppState>,
    Json(request): Json<PositionHealthRequest>,
) -> Result<impl IntoResponse, AppError> {
    let position = sqlx::query_as::<_, Position>(
        "SELECT * FROM positions WHERE id = $1"
    )
    .bind(&request.position_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(AppError::Database)?;

    let position = position.ok_or_else(|| 
        AppError::NotFound("Position not found".to_string())
    )?;

    // Mock current price for demonstration
    let mock_current_price = BigDecimal::from_i32(50000).unwrap();
    
    // For now, return basic health status
    let response_data = json!({
        "position_id": request.position_id,
        "is_healthy": true,
        "current_price": mock_current_price,
        "entry_price": position.entry_price,
        "margin": position.margin
    });

    Ok(Json(ApiResponse::success(response_data)))
}

// Request/Response types
#[derive(Deserialize)]
pub struct MarginRequest {
    pub entry_price: BigDecimal,
    pub size: BigDecimal,
    pub leverage: u8,
}

#[derive(Deserialize)]
pub struct PositionHealthRequest {
    pub position_id: String,
}

// Error handling
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::Database(ref e) => {
                log::error!("Database error: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Database error".to_string())
            },
            AppError::InvalidInput(ref message) => (StatusCode::BAD_REQUEST, message.clone()),
            AppError::NotFound(ref message) => (StatusCode::NOT_FOUND, message.clone()),
            AppError::Internal(ref message) => {
                log::error!("Internal error: {}", message);
                (StatusCode::INTERNAL_SERVER_ERROR, message.clone())
            },
            AppError::BadRequest(ref message) => (StatusCode::BAD_REQUEST, message.clone()),
            AppError::ServiceUnavailable(ref message) => (StatusCode::SERVICE_UNAVAILABLE, message.clone()),
        };

        let body = Json(json!({
            "error": error_message
        }));

        (status, body).into_response()
    }
}

// Modify an existing position
pub async fn modify_position(
    State(state): State<AppState>,
    Path(position_id): Path<String>,
    Json(payload): Json<ModifyPositionPayload>,
) -> Result<impl IntoResponse, AppError> {
    // Fetch existing position
    let position = sqlx::query_as::<_, Position>(
        "SELECT * FROM positions WHERE id = $1 AND status = 'open'"
    )
    .bind(&position_id)
    .fetch_optional(&state.pool)
    .await
    .map_err(AppError::Database)?;

    let position = position.ok_or_else(|| 
        AppError::NotFound("Open position not found".to_string())
    )?;

    let amount = BigDecimal::from_str(&payload.amount)
        .map_err(|_| AppError::BadRequest("Invalid amount".to_string()))?;

    let now = Utc::now();

    match payload.modification_type.as_str() {
        "increase_size" => {
            let new_size = &position.size + &amount;
            let new_entry_price = if let Some(price_str) = &payload.new_entry_price {
                let new_price = BigDecimal::from_str(price_str)
                    .map_err(|_| AppError::BadRequest("Invalid entry price".to_string()))?;
                // Calculate weighted average entry price
                let old_value = &position.size * &position.entry_price;
                let new_value = &amount * &new_price;
                let total_value = old_value + new_value;
                total_value / &new_size
            } else {
                position.entry_price.clone()
            };

            // Recalculate margin
            let new_margin = state.margin_calculator.calculate_required_margin(
                &new_entry_price,
                &new_size,
                position.leverage as u8,
            )?;

            sqlx::query(
                "UPDATE positions SET size = $1, entry_price = $2, margin = $3, updated_at = $4 WHERE id = $5"
            )
            .bind(&new_size)
            .bind(&new_entry_price)
            .bind(&new_margin)
            .bind(now)
            .bind(&position_id)
            .execute(&state.pool)
            .await
            .map_err(AppError::Database)?;

            // Record modification
            sqlx::query(
                "INSERT INTO position_modifications (position_id, user_id, modification_type, old_value, new_value, timestamp) VALUES ($1, $2, $3, $4, $5, $6)"
            )
            .bind(&position_id)
            .bind(&position.user_id)
            .bind("increase_size")
            .bind(&position.size)
            .bind(&new_size)
            .bind(now)
            .execute(&state.pool)
            .await
            .map_err(AppError::Database)?;

            Ok(Json(ApiResponse::success(json!({
                "position_id": position_id,
                "modification": "increase_size",
                "old_size": position.size,
                "new_size": new_size,
                "new_entry_price": new_entry_price,
                "new_margin": new_margin
            }))))
        },
        "decrease_size" => {
            if amount >= position.size {
                return Err(AppError::BadRequest("Cannot decrease by more than current size".to_string()));
            }
            let new_size = &position.size - &amount;
            
            // Recalculate margin for smaller position
            let new_margin = state.margin_calculator.calculate_required_margin(
                &position.entry_price,
                &new_size,
                position.leverage as u8,
            )?;

            // Calculate partial realized PnL (using mock current price)
            let mock_exit_price = &position.entry_price * BigDecimal::from_str("1.02").unwrap();
            let partial_pnl = if position.side.to_string() == "Long" {
                &amount * (&mock_exit_price - &position.entry_price)
            } else {
                &amount * (&position.entry_price - &mock_exit_price)
            };

            let current_pnl = position.realized_pnl.clone().unwrap_or(BigDecimal::from(0));
            let new_realized_pnl = &current_pnl + &partial_pnl;

            sqlx::query(
                "UPDATE positions SET size = $1, margin = $2, realized_pnl = $3, updated_at = $4 WHERE id = $5"
            )
            .bind(&new_size)
            .bind(&new_margin)
            .bind(&new_realized_pnl)
            .bind(now)
            .bind(&position_id)
            .execute(&state.pool)
            .await
            .map_err(AppError::Database)?;

            // Record modification
            sqlx::query(
                "INSERT INTO position_modifications (position_id, user_id, modification_type, old_value, new_value, timestamp) VALUES ($1, $2, $3, $4, $5, $6)"
            )
            .bind(&position_id)
            .bind(&position.user_id)
            .bind("decrease_size")
            .bind(&position.size)
            .bind(&new_size)
            .bind(now)
            .execute(&state.pool)
            .await
            .map_err(AppError::Database)?;

            Ok(Json(ApiResponse::success(json!({
                "position_id": position_id,
                "modification": "decrease_size",
                "old_size": position.size,
                "new_size": new_size,
                "partial_pnl_realized": partial_pnl,
                "new_margin": new_margin
            }))))
        },
        "add_margin" => {
            let new_margin = &position.margin + &amount;
            // Adding margin reduces effective leverage
            let position_value = &position.size * &position.entry_price;
            let new_leverage = (&position_value / &new_margin).to_string();

            sqlx::query(
                "UPDATE positions SET margin = $1, updated_at = $2 WHERE id = $3"
            )
            .bind(&new_margin)
            .bind(now)
            .bind(&position_id)
            .execute(&state.pool)
            .await
            .map_err(AppError::Database)?;

            // Record modification
            sqlx::query(
                "INSERT INTO position_modifications (position_id, user_id, modification_type, old_value, new_value, timestamp) VALUES ($1, $2, $3, $4, $5, $6)"
            )
            .bind(&position_id)
            .bind(&position.user_id)
            .bind("add_margin")
            .bind(&position.margin)
            .bind(&new_margin)
            .bind(now)
            .execute(&state.pool)
            .await
            .map_err(AppError::Database)?;

            Ok(Json(ApiResponse::success(json!({
                "position_id": position_id,
                "modification": "add_margin",
                "old_margin": position.margin,
                "new_margin": new_margin,
                "effective_leverage": new_leverage
            }))))
        },
        "remove_margin" => {
            if amount >= position.margin {
                return Err(AppError::BadRequest("Cannot remove more than current margin".to_string()));
            }
            let new_margin = &position.margin - &amount;
            
            // Check if removing margin would exceed max leverage
            let position_value = &position.size * &position.entry_price;
            let new_leverage_decimal = &position_value / &new_margin;
            let new_leverage_int = new_leverage_decimal.to_string().parse::<f64>().unwrap_or(1000.0);
            
            if new_leverage_int > 1000.0 {
                return Err(AppError::BadRequest("Removing margin would exceed max leverage of 1000x".to_string()));
            }

            sqlx::query(
                "UPDATE positions SET margin = $1, updated_at = $2 WHERE id = $3"
            )
            .bind(&new_margin)
            .bind(now)
            .bind(&position_id)
            .execute(&state.pool)
            .await
            .map_err(AppError::Database)?;

            // Record modification
            sqlx::query(
                "INSERT INTO position_modifications (position_id, user_id, modification_type, old_value, new_value, timestamp) VALUES ($1, $2, $3, $4, $5, $6)"
            )
            .bind(&position_id)
            .bind(&position.user_id)
            .bind("remove_margin")
            .bind(&position.margin)
            .bind(&new_margin)
            .bind(now)
            .execute(&state.pool)
            .await
            .map_err(AppError::Database)?;

            Ok(Json(ApiResponse::success(json!({
                "position_id": position_id,
                "modification": "remove_margin",
                "old_margin": position.margin,
                "new_margin": new_margin,
                "new_leverage": new_leverage_int
            }))))
        },
        _ => Err(AppError::BadRequest(format!(
            "Invalid modification type: {}. Use: increase_size, decrease_size, add_margin, remove_margin",
            payload.modification_type
        )))
    }
}

// WebSocket handler for real-time position updates
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_websocket(socket, state))
}

async fn handle_websocket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    
    // Send welcome message
    let welcome = json!({
        "type": "connected",
        "message": "WebSocket connected to Position Management System",
        "timestamp": Utc::now().to_rfc3339()
    });
    
    if sender.send(Message::Text(welcome.to_string())).await.is_err() {
        return;
    }

    // Spawn task to send periodic updates
    let state_clone = state.clone();
    let mut send_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
        
        loop {
            interval.tick().await;
            
            // Get all open positions count
            let open_count = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM positions WHERE status = 'open'"
            )
            .fetch_one(&state_clone.pool)
            .await
            .unwrap_or(0);

            // Mock price updates
            let price_update = json!({
                "type": "price_update",
                "data": {
                    "BTC/USD": 51000.0 + (rand::random::<f64>() * 1000.0 - 500.0),
                    "ETH/USD": 2800.0 + (rand::random::<f64>() * 100.0 - 50.0),
                    "SOL/USD": 120.0 + (rand::random::<f64>() * 10.0 - 5.0)
                },
                "open_positions": open_count,
                "timestamp": Utc::now().to_rfc3339()
            });

            if sender.send(Message::Text(price_update.to_string())).await.is_err() {
                break;
            }
        }
    });

    // Handle incoming messages
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Text(text) => {
                    // Parse subscription requests
                    if let Ok(request) = serde_json::from_str::<serde_json::Value>(&text) {
                        if request.get("type") == Some(&json!("subscribe")) {
                            log::info!("Client subscribed: {:?}", request.get("channel"));
                        }
                    }
                },
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    // Wait for either task to complete
    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }
}
