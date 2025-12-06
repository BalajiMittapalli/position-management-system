use anchor_lang::prelude::*;

// Precision constants for fixed-point arithmetic
pub const PRECISION_SCALE: u64 = 1_000_000; // 1e6 for price precision
pub const PERCENTAGE_SCALE: u64 = 10_000; // 1e4 for percentage precision (0.01%)

/// Calculate initial margin required for a position
/// Formula: size * entry_price / leverage
pub fn initial_margin(size: u64, entry_price: u64, leverage: u8) -> Result<u64> {
    require!(leverage > 0, crate::error::ValidationError::InvalidLeverage);
    
    let position_value = size
        .checked_mul(entry_price)
        .ok_or(crate::error::ValidationError::MathOverflow)?;
    
    let margin = position_value
        .checked_div(leverage as u64)
        .ok_or(crate::error::ValidationError::MathOverflow)?;
    
    Ok(margin)
}

/// Calculate maintenance margin from initial margin
/// Formula: initial_margin * maintenance_rate
pub fn maintenance_margin(initial_margin_amount: u64, maintenance_rate: f64) -> Result<u64> {
    let rate_scaled = (maintenance_rate * PERCENTAGE_SCALE as f64) as u64;
    
    let maintenance = initial_margin_amount
        .checked_mul(rate_scaled)
        .ok_or(crate::error::ValidationError::MathOverflow)?
        .checked_div(PERCENTAGE_SCALE)
        .ok_or(crate::error::ValidationError::MathOverflow)?;
    
    Ok(maintenance)
}

/// Calculate unrealized PnL for a position
/// Long: (mark_price - entry_price) * size
/// Short: (entry_price - mark_price) * size
pub fn unrealized_pnl(is_long: bool, size: u64, mark_price: u64, entry_price: u64) -> Result<i64> {
    let price_diff = if is_long {
        mark_price as i64 - entry_price as i64
    } else {
        entry_price as i64 - mark_price as i64
    };
    
    let pnl = price_diff
        .checked_mul(size as i64)
        .ok_or(crate::error::ValidationError::MathOverflow)?;
    
    Ok(pnl)
}

/// Calculate margin ratio for liquidation check
/// Formula: (margin + unrealized_pnl) / position_value
pub fn margin_ratio(margin: u64, unrealized_pnl: i64, position_value: u64) -> Result<i64> {
    require!(position_value > 0, crate::error::ValidationError::MathOverflow);
    
    let effective_margin = (margin as i64)
        .checked_add(unrealized_pnl)
        .ok_or(crate::error::ValidationError::MathOverflow)?;
    
    let ratio = effective_margin
        .checked_mul(PERCENTAGE_SCALE as i64)
        .ok_or(crate::error::ValidationError::MathOverflow)?
        .checked_div(position_value as i64)
        .ok_or(crate::error::ValidationError::MathOverflow)?;
    
    Ok(ratio)
}

/// Calculate liquidation price for long position
/// Formula: entry_price * (1 - 1/leverage + mmr)
pub fn liquidation_price_long(entry_price: u64, leverage: u8, mmr: f64) -> Result<u64> {
    require!(leverage > 0, crate::error::ValidationError::InvalidLeverage);
    
    let leverage_factor = PRECISION_SCALE / (leverage as u64);
    let mmr_scaled = (mmr * PRECISION_SCALE as f64) as u64;
    
    let adjustment = PRECISION_SCALE
        .checked_sub(leverage_factor)
        .ok_or(crate::error::ValidationError::MathOverflow)?
        .checked_add(mmr_scaled)
        .ok_or(crate::error::ValidationError::MathOverflow)?;
    
    let liquidation_price = entry_price
        .checked_mul(adjustment)
        .ok_or(crate::error::ValidationError::MathOverflow)?
        .checked_div(PRECISION_SCALE)
        .ok_or(crate::error::ValidationError::MathOverflow)?;
    
    Ok(liquidation_price)
}

/// Calculate liquidation price for short position
/// Formula: entry_price * (1 + 1/leverage - mmr)
pub fn liquidation_price_short(entry_price: u64, leverage: u8, mmr: f64) -> Result<u64> {
    require!(leverage > 0, crate::error::ValidationError::InvalidLeverage);
    
    let leverage_factor = PRECISION_SCALE / (leverage as u64);
    let mmr_scaled = (mmr * PRECISION_SCALE as f64) as u64;
    
    let adjustment = PRECISION_SCALE
        .checked_add(leverage_factor)
        .ok_or(crate::error::ValidationError::MathOverflow)?
        .checked_sub(mmr_scaled)
        .ok_or(crate::error::ValidationError::MathOverflow)?;
    
    let liquidation_price = entry_price
        .checked_mul(adjustment)
        .ok_or(crate::error::ValidationError::MathOverflow)?
        .checked_div(PRECISION_SCALE)
        .ok_or(crate::error::ValidationError::MathOverflow)?;
    
    Ok(liquidation_price)
}

/// Calculate weighted average entry price when adding to position
pub fn weighted_avg_entry_price(
    existing_size: u64,
    existing_entry_price: u64,
    new_size: u64,
    new_entry_price: u64,
) -> Result<u64> {
    let existing_value = existing_size
        .checked_mul(existing_entry_price)
        .ok_or(crate::error::ValidationError::MathOverflow)?;
    
    let new_value = new_size
        .checked_mul(new_entry_price)
        .ok_or(crate::error::ValidationError::MathOverflow)?;
    
    let total_value = existing_value
        .checked_add(new_value)
        .ok_or(crate::error::ValidationError::MathOverflow)?;
    
    let total_size = existing_size
        .checked_add(new_size)
        .ok_or(crate::error::ValidationError::MathOverflow)?;
    
    require!(total_size > 0, crate::error::ValidationError::MathOverflow);
    
    let weighted_avg = total_value
        .checked_div(total_size)
        .ok_or(crate::error::ValidationError::MathOverflow)?;
    
    Ok(weighted_avg)
}

/// Calculate position notional value
pub fn position_value(size: u64, price: u64) -> Result<u64> {
    let value = size
        .checked_mul(price)
        .ok_or(crate::error::ValidationError::MathOverflow)?;
    
    Ok(value)
}
