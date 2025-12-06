use std::sync::Arc;
use std::time::Duration;
use sqlx::PgPool;
use tokio::time;
use log::{info, error, debug, warn};
use bigdecimal::{BigDecimal, FromPrimitive};
use chrono::Utc;

use crate::models::{Position, PositionStatus, Side};
use crate::margin_calculator::MarginCalculator;

pub struct PositionMonitor {
    db_pool: PgPool,
    margin_calculator: Arc<MarginCalculator>,
}

impl PositionMonitor {
    pub fn new(db_pool: PgPool) -> Self {
        Self {
            db_pool,
            margin_calculator: Arc::new(MarginCalculator::new()),
        }
    }

    /// Start the position monitoring service
    pub async fn start_monitoring(&self) {
        info!("Starting position monitoring service...");
        
        let mut interval = time::interval(Duration::from_secs(30)); // Check every 30 seconds
        
        loop {
            interval.tick().await;
            
            if let Err(e) = self.monitor_positions().await {
                error!("Error in position monitoring: {}", e);
            }
        }
    }

    /// Monitor all active positions
    async fn monitor_positions(&self) -> Result<(), Box<dyn std::error::Error>> {
        debug!("Checking all active positions...");
        
        // Get all open positions
        let positions = sqlx::query_as::<_, Position>(
            "SELECT * FROM positions WHERE status = 'open'"
        )
        .fetch_all(&self.db_pool)
        .await?;

        debug!("Found {} open positions to monitor", positions.len());

        for position in positions {
            if let Err(e) = self.check_position_health(&position).await {
                error!("Error checking position {}: {}", position.id, e);
            }
        }

        Ok(())
    }

    /// Check the health of a specific position
    async fn check_position_health(&self, position: &Position) -> Result<(), Box<dyn std::error::Error>> {
        // Mock current price - in real implementation, this would come from price feeds
        let mock_current_price = self.get_mock_current_price(&position.symbol);
        
        // Calculate unrealized PnL
        let unrealized_pnl = self.calculate_unrealized_pnl(position, &mock_current_price)?;
        
        // Update position with current unrealized PnL
        self.update_position_pnl(&position.id, &unrealized_pnl).await?;
        
        // Check if position needs to be liquidated
        if self.should_liquidate_position(position, &unrealized_pnl)? {
            warn!("Position {} should be liquidated", position.id);
            self.liquidate_position(&position.id).await?;
        }

        debug!(
            "Position {} health check complete. Current PnL: {}", 
            position.id, unrealized_pnl
        );

        Ok(())
    }

    /// Get mock current price for testing
    fn get_mock_current_price(&self, symbol: &str) -> BigDecimal {
        match symbol {
            "BTC/USD" => BigDecimal::from_i32(45000).unwrap(), // Mock BTC price
            "ETH/USD" => BigDecimal::from_i32(3000).unwrap(),  // Mock ETH price
            "SOL/USD" => BigDecimal::from_i32(100).unwrap(),   // Mock SOL price
            _ => BigDecimal::from_i32(1000).unwrap(),          // Default mock price
        }
    }

    /// Calculate unrealized PnL for a position
    fn calculate_unrealized_pnl(
        &self,
        position: &Position,
        current_price: &BigDecimal,
    ) -> Result<BigDecimal, Box<dyn std::error::Error>> {
        let price_diff = match position.side {
            Side::Long => current_price - &position.entry_price,
            Side::Short => &position.entry_price - current_price,
        };
        
        let pnl = price_diff * &position.size;
        Ok(pnl)
    }

    /// Update position's unrealized PnL in the database
    async fn update_position_pnl(
        &self,
        position_id: &str,
        unrealized_pnl: &BigDecimal,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE positions SET unrealized_pnl = $1, updated_at = $2 WHERE id = $3",
            unrealized_pnl,
            Utc::now(),
            position_id
        )
        .execute(&self.db_pool)
        .await?;

        Ok(())
    }

    /// Determine if a position should be liquidated
    fn should_liquidate_position(
        &self,
        position: &Position,
        unrealized_pnl: &BigDecimal,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        // Calculate liquidation threshold (e.g., 90% of margin)
        let liquidation_threshold = &position.margin * BigDecimal::from_f64(0.9).unwrap();
        
        // Position should be liquidated if unrealized loss exceeds threshold
        let neg_threshold = -liquidation_threshold.clone();
        let should_liquidate = unrealized_pnl < &neg_threshold;
        
        if should_liquidate {
            warn!(
                "Position {} should be liquidated. Unrealized PnL: {}, Threshold: {}",
                position.id, unrealized_pnl, liquidation_threshold
            );
        }
        
        Ok(should_liquidate)
    }

    /// Liquidate a position
    async fn liquidate_position(&self, position_id: &str) -> Result<(), sqlx::Error> {
        info!("Liquidating position {}", position_id);
        
        sqlx::query!(
            "UPDATE positions SET status = 'liquidated', updated_at = $1 WHERE id = $2",
            Utc::now(),
            position_id
        )
        .execute(&self.db_pool)
        .await?;

        info!("Position {} has been liquidated", position_id);
        Ok(())
    }

    /// Get position statistics
    pub async fn get_position_stats(&self) -> Result<PositionStats, sqlx::Error> {
        let total_positions = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM positions"
        )
        .fetch_one(&self.db_pool)
        .await?;

        let open_positions = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM positions WHERE status = 'open'"
        )
        .fetch_one(&self.db_pool)
        .await?;

        let liquidated_positions = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM positions WHERE status = 'liquidated'"
        )
        .fetch_one(&self.db_pool)
        .await?;

        Ok(PositionStats {
            total_positions,
            open_positions,
            liquidated_positions,
        })
    }

    /// Health check for the monitoring service
    pub async fn health_check(&self) -> Result<HealthStatus, Box<dyn std::error::Error>> {
        // Check database connection
        let db_healthy = sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(&self.db_pool)
            .await
            .is_ok();

        let status = if db_healthy {
            "healthy".to_string()
        } else {
            "unhealthy".to_string()
        };

        Ok(HealthStatus {
            status,
            db_connected: db_healthy,
            timestamp: Utc::now(),
        })
    }
}

#[derive(Debug)]
pub struct PositionStats {
    pub total_positions: i64,
    pub open_positions: i64,
    pub liquidated_positions: i64,
}

#[derive(Debug, serde::Serialize)]
pub struct HealthStatus {
    pub status: String,
    pub db_connected: bool,
    pub timestamp: chrono::DateTime<Utc>,
}
