use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Type};
use chrono::{DateTime, Utc};
use bigdecimal::BigDecimal;
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[sqlx(type_name = "side_type", rename_all = "PascalCase")]
pub enum Side {
    Long,
    Short,
}

impl fmt::Display for Side {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Side::Long => write!(f, "Long"),
            Side::Short => write!(f, "Short"),
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[sqlx(type_name = "position_status", rename_all = "lowercase")]
pub enum PositionStatus {
    #[sqlx(rename = "open")]
    Open,
    #[sqlx(rename = "closed")]
    Closed,
    #[sqlx(rename = "liquidated")]
    Liquidated,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Position {
    pub id: String,
    pub user_id: String,
    pub symbol: String,
    pub side: Side,
    pub size: BigDecimal,
    pub entry_price: BigDecimal,
    pub leverage: i16,
    pub margin: BigDecimal,
    pub status: PositionStatus,
    pub stop_loss: Option<BigDecimal>,
    pub take_profit: Option<BigDecimal>,
    pub realized_pnl: Option<BigDecimal>,
    pub unrealized_pnl: Option<BigDecimal>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct PositionModification {
    pub id: i32,
    pub position_id: String,
    pub user_id: String,
    pub modification_type: String,
    pub old_value: Option<BigDecimal>,
    pub new_value: Option<BigDecimal>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct PnlSnapshot {
    pub id: i32,
    pub position_id: String,
    pub user_id: String,
    pub symbol: String,
    pub unrealized_pnl: BigDecimal,
    pub realized_pnl: BigDecimal,
    pub price_at_snapshot: BigDecimal,
    pub margin_ratio: BigDecimal,
    pub snapshot_type: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct UserTradingStats {
    pub id: i32,
    pub user_id: String,
    pub total_positions_opened: Option<i32>,
    pub total_positions_closed: Option<i32>,
    pub total_positions_liquidated: Option<i32>,
    pub total_volume: Option<BigDecimal>,
    pub total_realized_pnl: Option<BigDecimal>,
    pub win_rate: Option<BigDecimal>,
    pub avg_holding_time_hours: Option<BigDecimal>,
    pub largest_win: Option<BigDecimal>,
    pub largest_loss: Option<BigDecimal>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct UserRiskMetrics {
    pub id: i32,
    pub user_id: String,
    pub current_exposure: Option<BigDecimal>,
    pub max_leverage_used: Option<i16>,
    pub avg_leverage_used: Option<BigDecimal>,
    pub liquidation_risk_score: Option<BigDecimal>,
    pub var_95: Option<BigDecimal>,
    pub max_drawdown: Option<BigDecimal>,
    pub sharpe_ratio: Option<BigDecimal>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct FundingRate {
    pub symbol: String,
    pub rate: BigDecimal,
    pub next_funding_time: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Request/Response DTOs
#[derive(Debug, Deserialize)]
pub struct OpenPositionRequest {
    pub user_id: String,
    pub symbol: String,
    pub side: Side,
    pub size: String,
    pub entry_price: String,
    pub leverage: i16,
    pub stop_loss: Option<String>,
    pub take_profit: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ModifyPositionRequest {
    pub size: Option<String>,
    pub stop_loss: Option<String>,
    pub take_profit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionResponse {
    pub id: String,
    pub user_id: String,
    pub symbol: String,
    pub side: Side,
    pub size: String,
    pub entry_price: String,
    pub leverage: i16,
    pub margin: String,
    pub status: PositionStatus,
    pub stop_loss: Option<String>,
    pub take_profit: Option<String>,
    pub realized_pnl: Option<String>,
    pub unrealized_pnl: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Position> for PositionResponse {
    fn from(pos: Position) -> Self {
        Self {
            id: pos.id,
            user_id: pos.user_id,
            symbol: pos.symbol,
            side: pos.side,
            size: pos.size.to_string(),
            entry_price: pos.entry_price.to_string(),
            leverage: pos.leverage,
            margin: pos.margin.to_string(),
            status: pos.status,
            stop_loss: pos.stop_loss.map(|x| x.to_string()),
            take_profit: pos.take_profit.map(|x| x.to_string()),
            realized_pnl: pos.realized_pnl.map(|x| x.to_string()),
            unrealized_pnl: pos.unrealized_pnl.map(|x| x.to_string()),
            created_at: pos.created_at,
            updated_at: pos.updated_at,
        }
    }
}

// WebSocket Events
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type")]
pub enum WebSocketEvent {
    PositionUpdate { position: PositionResponse },
    PnlUpdate { position_id: String, unrealized_pnl: String, realized_pnl: String },
    MarginCall { position_id: String, required_margin: String },
    Liquidation { position_id: String, liquidation_price: String },
}

// Error types
#[derive(Debug)]
pub enum AppError {
    Database(sqlx::Error),
    InvalidInput(String),
    NotFound(String),
    Internal(String),
    BadRequest(String),
    ServiceUnavailable(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Database(err) => write!(f, "Database error: {}", err),
            AppError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            AppError::NotFound(msg) => write!(f, "Not found: {}", msg),
            AppError::Internal(msg) => write!(f, "Internal error: {}", msg),
            AppError::BadRequest(msg) => write!(f, "Bad request: {}", msg),
            AppError::ServiceUnavailable(msg) => write!(f, "Service unavailable: {}", msg),
        }
    }
}

impl std::error::Error for AppError {}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::Database(err)
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::Internal(err.to_string())
    }
}

// Response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
        }
    }
}

// Additional request types needed by routes
#[derive(Debug, Deserialize)]
pub struct PositionRequest {
    pub user_id: String,
    pub symbol: String,
    pub side: Side,
    pub size: BigDecimal,
    pub entry_price: BigDecimal,
    pub leverage: i16,
    pub stop_loss: Option<BigDecimal>,
    pub take_profit: Option<BigDecimal>,
}
