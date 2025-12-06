use anchor_lang::prelude::*;
use crate::state::*;
use crate::calculations::*;
use crate::error::ValidationError;

#[derive(Accounts)]
#[instruction(symbol: String)]
pub struct ClosePosition<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    
    #[account(
        mut,
        seeds = [USER_ACCOUNT_SEED, user.key().as_ref()],
        bump,
        constraint = user_account.owner == user.key() @ ValidationError::Unauthorized
    )]
    pub user_account: Account<'info, UserAccount>,
    
    #[account(
        mut,
        close = user,
        seeds = [POSITION_SEED, user.key().as_ref(), symbol.as_bytes()],
        bump = position.bump,
        constraint = position.owner == user.key() @ ValidationError::Unauthorized
    )]
    pub position: Account<'info, Position>,
}

pub fn handler(
    ctx: Context<ClosePosition>,
    _symbol: String,
    exit_price: u64,
) -> Result<()> {
    let user_account = &mut ctx.accounts.user_account;
    let position = &ctx.accounts.position;
    
    require!(exit_price > 0, ValidationError::MathOverflow);
    require!(position.size > 0, ValidationError::ZeroPositionSize);
    
    // Calculate final realized PnL
    let is_long = matches!(position.side, Side::Long);
    let final_pnl = unrealized_pnl(is_long, position.size, exit_price, position.entry_price)?;
    
    // Total PnL includes any existing realized PnL and funding
    let total_pnl = position.realized_pnl
        .checked_add(final_pnl)
        .ok_or(ValidationError::MathOverflow)?
        .checked_sub(position.funding_accrued)
        .ok_or(ValidationError::MathOverflow)?;
    
    // Calculate leftover margin after accounting for PnL
    let leftover_margin = if total_pnl >= 0 {
        position.margin
            .checked_add(total_pnl as u64)
            .ok_or(ValidationError::MathOverflow)?
    } else {
        let loss = (-total_pnl) as u64;
        if loss >= position.margin {
            0 // Total loss exceeds margin
        } else {
            position.margin
                .checked_sub(loss)
                .ok_or(ValidationError::MathOverflow)?
        }
    };
    
    // Update user account
    user_account.locked_collateral = user_account.locked_collateral
        .checked_sub(position.margin)
        .ok_or(ValidationError::MathOverflow)?;
    
    user_account.total_pnl = user_account.total_pnl
        .checked_add(total_pnl)
        .ok_or(ValidationError::MathOverflow)?;
    
    user_account.position_count = user_account.position_count
        .checked_sub(1)
        .ok_or(ValidationError::MathOverflow)?;
    
    // If there's leftover margin, it gets returned to available collateral
    if leftover_margin > 0 {
        user_account.total_collateral = user_account.total_collateral
            .checked_add(leftover_margin)
            .ok_or(ValidationError::MathOverflow)?;
    }
    
    // Emit event before account is closed
    emit!(PositionClosed {
        user: position.owner,
        symbol: position.symbol.clone(),
        side: position.side.clone(),
        size: position.size,
        entry_price: position.entry_price,
        exit_price,
        realized_pnl: total_pnl,
        margin_returned: leftover_margin,
    });
    
    msg!(
        "Position closed: {} {} {} at {} with PnL {}",
        if matches!(position.side, Side::Long) { "LONG" } else { "SHORT" },
        position.size,
        position.symbol,
        exit_price,
        total_pnl
    );
    
    // Position account is automatically closed due to the close constraint
    Ok(())
}

#[event]
pub struct PositionClosed {
    pub user: Pubkey,
    pub symbol: String,
    pub side: Side,
    pub size: u64,
    pub entry_price: u64,
    pub exit_price: u64,
    pub realized_pnl: i64,
    pub margin_returned: u64,
}
