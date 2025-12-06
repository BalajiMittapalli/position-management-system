use bigdecimal::BigDecimal;
use anyhow::Result;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct MarginCalculator {
    // Leverage tiers: leverage -> maintenance margin rate
    leverage_tiers: HashMap<u8, BigDecimal>,
    // Symbol-specific margin requirements
    symbol_margins: HashMap<String, BigDecimal>,
}

#[derive(Debug, Clone)]
pub struct MarginRequirement {
    pub initial_margin: BigDecimal,
    pub maintenance_margin: BigDecimal,
    pub liquidation_price: BigDecimal,
}

impl MarginCalculator {
    pub fn new() -> Self {
        let mut leverage_tiers = HashMap::new();
        // Standard leverage tiers with maintenance margin rates
        leverage_tiers.insert(1, BigDecimal::from(100)); // 100% (no leverage)
        leverage_tiers.insert(2, BigDecimal::from(50));  // 50%
        leverage_tiers.insert(5, BigDecimal::from(20));  // 20%
        leverage_tiers.insert(10, BigDecimal::from(10)); // 10%
        leverage_tiers.insert(20, BigDecimal::from(5));  // 5%
        leverage_tiers.insert(50, BigDecimal::from(2));  // 2%
        leverage_tiers.insert(100, BigDecimal::from(1)); // 1%

        let mut symbol_margins = HashMap::new();
        // Default margin requirements for different symbols
        symbol_margins.insert("BTC/USD".to_string(), BigDecimal::from(1)); // 1%
        symbol_margins.insert("ETH/USD".to_string(), BigDecimal::from(2)); // 2%
        symbol_margins.insert("SOL/USD".to_string(), BigDecimal::from(5)); // 5%

        Self {
            leverage_tiers,
            symbol_margins,
        }
    }

    pub fn calculate_margin_requirement(
        &self,
        symbol: &str,
        size: &BigDecimal,
        entry_price: &BigDecimal,
        leverage: u8,
        side: &str,
    ) -> Result<MarginRequirement> {
        let position_value = size * entry_price;
        
        // Get base margin rate for symbol
        let base_margin_rate = self.symbol_margins
            .get(symbol)
            .unwrap_or(&BigDecimal::from(5)) // Default 5%
            .clone();

        // Calculate initial margin (position_value / leverage)
        let initial_margin = &position_value / BigDecimal::from(leverage as i32);

        // Get maintenance margin rate for leverage tier
        let maintenance_rate = self.leverage_tiers
            .get(&leverage)
            .unwrap_or(&BigDecimal::from(10)) // Default 10%
            .clone();

        let maintenance_margin = &position_value * &maintenance_rate / BigDecimal::from(100);

        // Calculate liquidation price
        let liquidation_price = self.calculate_liquidation_price(
            entry_price,
            &maintenance_margin,
            &position_value,
            side,
        )?;

        Ok(MarginRequirement {
            initial_margin,
            maintenance_margin,
            liquidation_price,
        })
    }

    pub fn calculate_liquidation_price(
        &self,
        entry_price: &BigDecimal,
        maintenance_margin: &BigDecimal,
        position_value: &BigDecimal,
        side: &str,
    ) -> Result<BigDecimal> {
        let margin_ratio = maintenance_margin / position_value;
        
        let liquidation_price = match side.to_lowercase().as_str() {
            "long" => {
                // For long positions: liquidation_price = entry_price * (1 - margin_ratio)
                entry_price * (BigDecimal::from(1) - &margin_ratio)
            }
            "short" => {
                // For short positions: liquidation_price = entry_price * (1 + margin_ratio)
                entry_price * (BigDecimal::from(1) + &margin_ratio)
            }
            _ => return Err(anyhow::anyhow!("Invalid side: {}", side)),
        };

        Ok(liquidation_price)
    }

    pub fn calculate_pnl(
        &self,
        entry_price: &BigDecimal,
        current_price: &BigDecimal,
        size: &BigDecimal,
        side: &str,
    ) -> Result<BigDecimal> {
        let price_diff = current_price - entry_price;
        
        let pnl = match side.to_lowercase().as_str() {
            "long" => size * &price_diff,
            "short" => size * (-&price_diff),
            _ => return Err(anyhow::anyhow!("Invalid side: {}", side)),
        };

        Ok(pnl)
    }

    pub fn is_position_liquidatable(
        &self,
        entry_price: &BigDecimal,
        current_price: &BigDecimal,
        maintenance_margin: &BigDecimal,
        position_value: &BigDecimal,
        side: &str,
    ) -> Result<bool> {
        let liquidation_price = self.calculate_liquidation_price(
            entry_price,
            maintenance_margin,
            position_value,
            side,
        )?;

        let is_liquidatable = match side.to_lowercase().as_str() {
            "long" => current_price <= &liquidation_price,
            "short" => current_price >= &liquidation_price,
            _ => return Err(anyhow::anyhow!("Invalid side: {}", side)),
        };

        Ok(is_liquidatable)
    }

    pub fn calculate_max_leverage(&self, symbol: &str) -> u8 {
        // Return maximum allowed leverage for symbol
        match symbol {
            "BTC/USD" => 100,
            "ETH/USD" => 50,
            "SOL/USD" => 20,
            _ => 10, // Conservative default
        }
    }

    pub fn add_symbol_margin(&mut self, symbol: String, margin_rate: BigDecimal) {
        self.symbol_margins.insert(symbol, margin_rate);
    }

    pub fn add_leverage_tier(&mut self, leverage: u8, maintenance_rate: BigDecimal) {
        self.leverage_tiers.insert(leverage, maintenance_rate);
    }

    // Calculate required margin for a position modification
    pub fn calculate_margin_delta(
        &self,
        current_size: &BigDecimal,
        new_size: &BigDecimal,
        entry_price: &BigDecimal,
        leverage: u8,
    ) -> Result<BigDecimal> {
        let current_position_value = current_size * entry_price;
        let new_position_value = new_size * entry_price;
        
        let current_required = &current_position_value / BigDecimal::from(leverage as i32);
        let new_required = &new_position_value / BigDecimal::from(leverage as i32);
        
        Ok(&new_required - &current_required)
    }

    /// Calculate required margin for a position given size, entry price, and leverage
    pub fn calculate_required_margin(
        &self,
        entry_price: &BigDecimal,
        size: &BigDecimal,
        leverage: u8,
    ) -> Result<BigDecimal> {
        let position_value = size * entry_price;
        let margin = &position_value / BigDecimal::from(leverage as i32);
        Ok(margin)
    }

    // Risk assessment for a position
    pub fn assess_position_risk(
        &self,
        entry_price: &BigDecimal,
        current_price: &BigDecimal,
        size: &BigDecimal,
        margin: &BigDecimal,
        leverage: u8,
        side: &str,
    ) -> Result<f64> {
        let position_value = size * entry_price;
        let pnl = self.calculate_pnl(entry_price, current_price, size, side)?;
        
        // Calculate current margin ratio
        let current_equity = margin + &pnl;
        let margin_ratio = if position_value > BigDecimal::from(0) {
            &current_equity / &position_value
        } else {
            BigDecimal::from(0)
        };

        // Risk score: 1.0 = no risk, 0.0 = high risk
        let risk_score = margin_ratio.to_string().parse::<f64>().unwrap_or(0.0);
        Ok(risk_score.max(0.0).min(1.0))
    }
}
