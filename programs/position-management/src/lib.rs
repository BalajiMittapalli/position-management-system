use anchor_lang::prelude::*;

declare_id!("3BpQ5UZ3B3jK4SioSf31d2fLaSV3jDeu77ZE3EeVyraf");

mod state;
mod calculations;
mod error;
mod instructions;

use state::*;
use instructions::*;

#[program]
pub mod position_management {
    use super::*;

    /// Open a new leveraged position
    pub fn open_position(
        ctx: Context<OpenPosition>,
        symbol: String,
        side: Side,
        size: u64,
        leverage: u8,
        entry_price: u64,
    ) -> Result<()> {
        instructions::open_position::handler(ctx, symbol, side, size, leverage, entry_price)
    }

    /// Modify an existing position (increase/decrease size, add/remove margin)
    pub fn modify_position(
        ctx: Context<ModifyPosition>,
        symbol: String,
        modification_type: ModificationType,
        amount: u64,
        new_entry_price: Option<u64>,
    ) -> Result<()> {
        instructions::modify_position::handler(ctx, symbol, modification_type, amount, new_entry_price)
    }

    /// Close a position and realize PnL
    pub fn close_position(
        ctx: Context<ClosePosition>,
        symbol: String,
        exit_price: u64,
    ) -> Result<()> {
        instructions::close_position::handler(ctx, symbol, exit_price)
    }
}
