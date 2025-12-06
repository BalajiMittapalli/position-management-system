use bigdecimal::Zero;
use bigdecimal::{BigDecimal, FromPrimitive, Signed};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

/// Utility functions for the position management backend

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceData {
    pub symbol: String,
    pub price: BigDecimal,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketData {
    pub bid: BigDecimal,
    pub ask: BigDecimal,
    pub last_price: BigDecimal,
    pub volume_24h: BigDecimal,
    pub timestamp: DateTime<Utc>,
}

/// Convert string to BigDecimal with error handling
pub fn parse_decimal(value: &str) -> Result<BigDecimal, String> {
    BigDecimal::from_str(value)
        .map_err(|e| format!("Failed to parse decimal '{}': {}", value, e))
}

/// Convert f64 to BigDecimal with error handling
pub fn f64_to_decimal(value: f64) -> Result<BigDecimal, String> {
    BigDecimal::from_f64(value)
        .ok_or_else(|| format!("Failed to convert f64 '{}' to BigDecimal", value))
}

/// Calculate percentage change between two values
pub fn calculate_percentage_change(old_value: &BigDecimal, new_value: &BigDecimal) -> BigDecimal {
    if old_value.is_zero() {
        return BigDecimal::from_i32(0).unwrap();
    }
    
    ((new_value - old_value) / old_value) * BigDecimal::from_i32(100).unwrap()
}

/// Calculate absolute value of BigDecimal
pub fn abs_decimal(value: &BigDecimal) -> BigDecimal {
    if value.is_negative() {
        -value
    } else {
        value.clone()
    }
}

/// Format BigDecimal to string with specified decimal places
pub fn format_decimal(value: &BigDecimal, decimal_places: u32) -> String {
    format!("{:.1$}", value, decimal_places as usize)
}

/// Generate a unique transaction ID
pub fn generate_tx_id() -> String {
    format!("tx_{}", Uuid::new_v4().simple())
}

/// Validate if a string is a valid Solana public key (basic format check)
pub fn validate_solana_pubkey(pubkey_str: &str) -> bool {
    // Base58 check: Solana pubkeys are 32-44 characters, base58 encoded
    pubkey_str.len() >= 32 && pubkey_str.len() <= 44 && 
    pubkey_str.chars().all(|c| {
        c.is_ascii_alphanumeric() && c != '0' && c != 'O' && c != 'I' && c != 'l'
    })
}
/// Get current timestamp
pub fn current_timestamp() -> DateTime<Utc> {
    Utc::now()
}

/// Convert timestamp to unix seconds
pub fn timestamp_to_unix_seconds(timestamp: &DateTime<Utc>) -> i64 {
    timestamp.timestamp()
}

/// Convert unix seconds to timestamp
pub fn unix_seconds_to_timestamp(unix_seconds: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(unix_seconds, 0).unwrap_or_else(Utc::now)
}

/// Calculate funding rate based on interest rates
pub fn calculate_funding_rate(
    base_rate: &BigDecimal,
    premium_index: &BigDecimal,
    funding_interval_hours: u32,
) -> BigDecimal {
    let hours_per_day = BigDecimal::from_i32(24).unwrap();
    let interval = BigDecimal::from_i32(funding_interval_hours as i32).unwrap();
    let daily_rate = base_rate + premium_index;
    
    &daily_rate * &interval / &hours_per_day
}

/// Calculate liquidation price for a position
pub fn calculate_liquidation_price(
    entry_price: &BigDecimal,
    side: &str,
    leverage: i16,
    maintenance_margin_rate: &BigDecimal,
) -> Result<BigDecimal, String> {
    let leverage_decimal = BigDecimal::from_i16(leverage)
        .ok_or("Invalid leverage")?;
    
    let one = BigDecimal::from_i32(1).unwrap();
    let margin_factor = &one - maintenance_margin_rate;
    
    match side {
        "long" => {
            // For long positions: liquidation_price = entry_price * (1 - (margin_factor / leverage))
            let factor = &one - (&margin_factor / &leverage_decimal);
            Ok(entry_price * &factor)
        }
        "short" => {
            // For short positions: liquidation_price = entry_price * (1 + (margin_factor / leverage))
            let factor = &one + (&margin_factor / &leverage_decimal);
            Ok(entry_price * &factor)
        }
        _ => Err("Invalid position side".to_string())
    }
}

/// Calculate position size in base currency
pub fn calculate_position_value(size: &BigDecimal, price: &BigDecimal) -> BigDecimal {
    size * price
}

/// Calculate required margin for a position
pub fn calculate_required_margin(
    size: &BigDecimal,
    price: &BigDecimal,
    leverage: i16,
) -> Result<BigDecimal, String> {
    let leverage_decimal = BigDecimal::from_i16(leverage)
        .ok_or("Invalid leverage")?;
    
    let position_value = calculate_position_value(size, price);
    Ok(position_value / leverage_decimal)
}

/// Validate position parameters
pub fn validate_position_params(
    size: &BigDecimal,
    leverage: i16,
    symbol: &str,
) -> Result<(), String> {
    // Check if size is positive
    if size <= &BigDecimal::from_i32(0).unwrap() {
        return Err("Position size must be positive".to_string());
    }
    
    // Check leverage limits (1x to 100x)
    if leverage < 1 || leverage > 100 {
        return Err("Leverage must be between 1 and 100".to_string());
    }
    
    // Check symbol format (simple validation)
    if symbol.is_empty() || symbol.len() > 20 {
        return Err("Invalid symbol format".to_string());
    }
    
    Ok(())
}

/// Calculate maximum position size based on available balance
pub fn calculate_max_position_size(
    available_balance: &BigDecimal,
    price: &BigDecimal,
    leverage: i16,
) -> Result<BigDecimal, String> {
    let leverage_decimal = BigDecimal::from_i16(leverage)
        .ok_or("Invalid leverage")?;
    
    // max_size = (available_balance * leverage) / price
    Ok((available_balance * leverage_decimal) / price)
}

/// Risk management utilities
pub mod risk {
    use super::*;

    /// Calculate Value at Risk (VaR) for a position
    pub fn calculate_var(
        position_value: &BigDecimal,
        volatility: &BigDecimal,
        confidence_level: f64, // e.g., 0.95 for 95% confidence
        time_horizon_days: u32,
    ) -> Result<BigDecimal, String> {
        let z_score = match confidence_level {
            0.90 => 1.28,
            0.95 => 1.65,
            0.99 => 2.33,
            _ => return Err("Unsupported confidence level".to_string()),
        };
        
        let time_factor = (time_horizon_days as f64).sqrt();
        let z_decimal = BigDecimal::from_f64(z_score * time_factor)
            .ok_or("Failed to convert z-score")?;
        
        Ok(position_value * volatility * z_decimal)
    }

    /// Calculate portfolio correlation adjustment
    pub fn calculate_correlation_adjustment(
        correlation: f64,
        portfolio_var: &BigDecimal,
        new_position_var: &BigDecimal,
    ) -> Result<BigDecimal, String> {
        let correlation_decimal = BigDecimal::from_f64(correlation)
            .ok_or("Failed to convert correlation")?;
        
        let two = BigDecimal::from_i32(2).unwrap();
        let correlation_term = &two * &correlation_decimal * portfolio_var * new_position_var;
        
        Ok(correlation_term.sqrt().unwrap_or_else(|| BigDecimal::from_i32(0).unwrap()))
    }
}

/// Price feed utilities
pub mod price_feed {
    use super::*;

    /// Mock price feed for testing
    pub async fn get_mock_price(symbol: &str) -> Result<PriceData, String> {
        // This would typically connect to real price feeds like Pyth, Switchboard, etc.
        let mock_prices = match symbol {
            "BTC/USD" => 45000.0,
            "ETH/USD" => 3000.0,
            "SOL/USD" => 100.0,
            _ => return Err(format!("Price not available for symbol: {}", symbol)),
        };
        
        Ok(PriceData {
            symbol: symbol.to_string(),
            price: BigDecimal::from_f64(mock_prices)
                .ok_or("Failed to convert mock price")?,
            timestamp: Utc::now(),
        })
    }

    /// Calculate TWAP (Time-Weighted Average Price)
    pub fn calculate_twap(prices: &[PriceData]) -> Option<BigDecimal> {
        if prices.is_empty() {
            return None;
        }

        let total: BigDecimal = prices.iter().map(|p| &p.price).sum();
        let count = BigDecimal::from_usize(prices.len())?;
        
        Some(total / count)
    }
}

/// Blockchain utilities
pub mod blockchain {
    /// Validate a Solana public key string format
    pub fn validate_pubkey(pubkey_str: &str) -> bool {
        // Base58 check: Solana pubkeys are 32-44 characters
        pubkey_str.len() >= 32 && pubkey_str.len() <= 44 && 
        pubkey_str.chars().all(|c| {
            c.is_ascii_alphanumeric() && c != '0' && c != 'O' && c != 'I' && c != 'l'
        })
    }

    /// Validate transaction signature format
    pub fn validate_tx_signature(signature: &str) -> bool {
        signature.len() >= 86 && signature.len() <= 88 && 
        signature.chars().all(|c| c.is_ascii_alphanumeric())
    }
}

/// Database utilities
pub mod db_utils {
    use super::*;

    /// Generate a database-safe UUID string
    pub fn generate_uuid_string() -> String {
        Uuid::new_v4().to_string()
    }

    /// Validate UUID format
    pub fn validate_uuid(uuid_str: &str) -> bool {
        Uuid::from_str(uuid_str).is_ok()
    }

    /// Convert UTC timestamp to database-compatible format
    pub fn timestamp_to_db_string(timestamp: &DateTime<Utc>) -> String {
        timestamp.format("%Y-%m-%d %H:%M:%S UTC").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_decimal() {
        assert!(parse_decimal("123.45").is_ok());
        assert!(parse_decimal("invalid").is_err());
    }

    #[test]
    fn test_calculate_percentage_change() {
        let old_val = BigDecimal::from_i32(100).unwrap();
        let new_val = BigDecimal::from_i32(110).unwrap();
        let change = calculate_percentage_change(&old_val, &new_val);
        assert_eq!(change, BigDecimal::from_i32(10).unwrap());
    }

    #[test]
    fn test_validate_position_params() {
        let size = BigDecimal::from_i32(10).unwrap();
        assert!(validate_position_params(&size, 10, "BTC/USD").is_ok());
        
        let zero_size = BigDecimal::from_i32(0).unwrap();
        assert!(validate_position_params(&zero_size, 10, "BTC/USD").is_err());
        
        assert!(validate_position_params(&size, 0, "BTC/USD").is_err());
        assert!(validate_position_params(&size, 101, "BTC/USD").is_err());
    }

    #[test]
    fn test_calculate_liquidation_price() {
        let entry_price = BigDecimal::from_i32(1000).unwrap();
        let maintenance_margin = BigDecimal::from_str("0.05").unwrap(); // 5%
        
        let liq_price = calculate_liquidation_price(&entry_price, "long", 10, &maintenance_margin);
        assert!(liq_price.is_ok());
        
        let liq_price = calculate_liquidation_price(&entry_price, "invalid", 10, &maintenance_margin);
        assert!(liq_price.is_err());
    }
}
