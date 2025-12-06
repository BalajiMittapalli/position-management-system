use anchor_lang::prelude::*;

// PDA seed constants
pub const POSITION_SEED: &[u8] = b"position";
pub const USER_ACCOUNT_SEED: &[u8] = b"user_account";

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Debug, PartialEq)]
pub enum Side {
    Long,
    Short,
}

#[account]
pub struct Position {
    pub owner: Pubkey,
    pub symbol: String,
    pub side: Side,
    pub size: u64,
    pub entry_price: u64,
    pub margin: u64,
    pub leverage: u8,
    pub unrealized_pnl: i64,
    pub realized_pnl: i64,
    pub funding_accrued: i64,
    pub liquidation_price: u64,
    pub last_update: i64,
    pub bump: u8,
}

#[account]
pub struct UserAccount {
    pub owner: Pubkey,
    pub total_collateral: u64,
    pub locked_collateral: u64,
    pub total_pnl: i64,
    pub position_count: u32,
    pub bump: u8,
}

// Leverage tier configuration
#[derive(Debug, Clone, Copy)]
pub struct LeverageTier {
    pub max_leverage: u16,
    pub initial_margin_rate: f64,
    pub maintenance_margin_rate: f64,
    pub max_position_size: u64,
}

pub const LEVERAGE_TIERS: [LeverageTier; 5] = [
    LeverageTier {
        max_leverage: 20,
        initial_margin_rate: 0.05,
        maintenance_margin_rate: 0.025,
        max_position_size: u64::MAX,
    },
    LeverageTier {
        max_leverage: 50,
        initial_margin_rate: 0.02,
        maintenance_margin_rate: 0.01,
        max_position_size: 100_000,
    },
    LeverageTier {
        max_leverage: 100,
        initial_margin_rate: 0.01,
        maintenance_margin_rate: 0.005,
        max_position_size: 50_000,
    },
    LeverageTier {
        max_leverage: 500,
        initial_margin_rate: 0.005,
        maintenance_margin_rate: 0.0025,
        max_position_size: 20_000,
    },
    LeverageTier {
        max_leverage: 1000,
        initial_margin_rate: 0.002,
        maintenance_margin_rate: 0.001,
        max_position_size: 5_000,
    },
];

impl Position {
    pub const LEN: usize = 8 + // discriminator
        32 + // owner
        4 + 64 + // symbol (string with length)
        1 + // side
        8 + // size
        8 + // entry_price
        8 + // margin
        1 + // leverage
        8 + // unrealized_pnl
        8 + // realized_pnl
        8 + // funding_accrued
        8 + // liquidation_price
        8 + // last_update
        1; // bump
}

impl UserAccount {
    pub const LEN: usize = 8 + // discriminator
        32 + // owner
        8 + // total_collateral
        8 + // locked_collateral
        8 + // total_pnl
        4 + // position_count
        1; // bump
}

use crate::error::ValidationError;

/// Get the appropriate leverage tier based on leverage and position size
pub fn get_leverage_tier(leverage: u16, position_size: u64) -> Result<LeverageTier> {
    for tier in LEVERAGE_TIERS.iter() {
        if leverage <= tier.max_leverage && position_size <= tier.max_position_size {
            return Ok(*tier);
        }
    }
    
    err!(ValidationError::LeverageExceeded)
}
