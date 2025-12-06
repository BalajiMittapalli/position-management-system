use sqlx::Row;
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use anyhow::Result;
use uuid::Uuid;
use std::collections::HashMap;

use crate::models::{Position, PnlSnapshot};

#[derive(Debug, Clone)]
pub struct PnlTracker {
    pool: PgPool,
}

#[derive(Debug, Clone)]
pub struct PnlSummary {
    pub total_realized_pnl: BigDecimal,
    pub total_unrealized_pnl: BigDecimal,
    pub daily_pnl: BigDecimal,
    pub weekly_pnl: BigDecimal,
    pub monthly_pnl: BigDecimal,
    pub position_count: i32,
    pub winning_positions: i32,
    pub losing_positions: i32,
    pub win_rate: f64,
}

impl PnlTracker {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_pnl_snapshot(
        &self,
        user_id: &str,
        total_pnl: &BigDecimal,
        realized_pnl: &BigDecimal,
        unrealized_pnl: &BigDecimal,
    ) -> Result<PnlSnapshot> {
        let snapshot = sqlx::query_as::<_, PnlSnapshot>(
            r#"
            INSERT INTO pnl_snapshots (id, user_id, total_pnl, realized_pnl, unrealized_pnl, timestamp)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#
        )
        .bind(Uuid::new_v4().to_string())
        .bind(user_id)
        .bind(total_pnl)
        .bind(realized_pnl)
        .bind(unrealized_pnl)
        .bind(Utc::now())
        .fetch_one(&self.pool)
        .await?;

        Ok(snapshot)
    }

    pub async fn get_user_pnl_summary(&self, user_id: &str) -> Result<PnlSummary> {
        let positions = sqlx::query_as::<_, Position>(
            "SELECT * FROM positions WHERE user_id = $1"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let mut total_realized_pnl = BigDecimal::from(0);
        let mut total_unrealized_pnl = BigDecimal::from(0);
        let mut winning_positions = 0;
        let mut losing_positions = 0;

        for position in &positions {
            if let Some(realized) = &position.realized_pnl {
                total_realized_pnl += realized;
                if realized > &BigDecimal::from(0) {
                    winning_positions += 1;
                } else if realized < &BigDecimal::from(0) {
                    losing_positions += 1;
                }
            }

            if let Some(unrealized) = &position.unrealized_pnl {
                total_unrealized_pnl += unrealized;
            }
        }

        let daily_pnl = self.get_pnl_for_period(user_id, 1).await?;
        let weekly_pnl = self.get_pnl_for_period(user_id, 7).await?;
        let monthly_pnl = self.get_pnl_for_period(user_id, 30).await?;

        let total_closed_positions = winning_positions + losing_positions;
        let win_rate = if total_closed_positions > 0 {
            winning_positions as f64 / total_closed_positions as f64
        } else {
            0.0
        };

        Ok(PnlSummary {
            total_realized_pnl,
            total_unrealized_pnl,
            daily_pnl,
            weekly_pnl,
            monthly_pnl,
            position_count: positions.len() as i32,
            winning_positions,
            losing_positions,
            win_rate,
        })
    }

    async fn get_pnl_for_period(&self, user_id: &str, days: i32) -> Result<BigDecimal> {
        let result = sqlx::query_scalar::<_, Option<BigDecimal>>(
            "SELECT COALESCE(SUM(realized_pnl), 0) FROM positions WHERE user_id = $1 AND updated_at >= NOW() - INTERVAL '1 day' * $2"
        )
        .bind(user_id)
        .bind(days)
        .fetch_one(&self.pool)
        .await?
        .unwrap_or(BigDecimal::from(0));

        Ok(result)
    }

    pub async fn get_symbol_performance(&self, user_id: &str) -> Result<HashMap<String, BigDecimal>> {
        let result = sqlx::query(
            "SELECT symbol, COALESCE(SUM(COALESCE(realized_pnl, 0) + COALESCE(unrealized_pnl, 0)), 0) as total_pnl FROM positions WHERE user_id = $1 GROUP BY symbol"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        let mut performance = HashMap::new();
        for row in result {
            let symbol: String = row.get("symbol");
            let pnl: BigDecimal = row.get("total_pnl");
            performance.insert(symbol, pnl);
        }

        Ok(performance)
    }
}
