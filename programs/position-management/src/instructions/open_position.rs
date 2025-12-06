use anchor_lang::prelude::*;
use crate::state::*;
use crate::calculations::*;
use crate::error::ValidationError;

#[derive(Accounts)]
#[instruction(symbol: String)]
pub struct OpenPosition<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    
    #[account(
        mut,
        seeds = [USER_ACCOUNT_SEED, user.key().as_ref()],
        bump = user_account.bump,
        constraint = user_account.owner == user.key() @ ValidationError::Unauthorized
    )]
    pub user_account: Account<'info, UserAccount>,
    
    #[account(
        init,
        payer = user,
        space = Position::LEN,
        seeds = [POSITION_SEED, user.key().as_ref(), symbol.as_bytes()],
        bump
    )]
    pub position: Account<'info, Position>,
    
    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<OpenPosition>,
    symbol: String,
    side: Side,
    size: u64,
    leverage: u8,
    entry_price: u64,
) -> Result<()> {
    require!(size > 0, ValidationError::ZeroPositionSize);
    require!(leverage > 0 && leverage <= 255, ValidationError::InvalidLeverage);
    require!(entry_price > 0, ValidationError::MathOverflow);
    
    let user_account = &mut ctx.accounts.user_account;
    let position = &mut ctx.accounts.position;
    let user_key = ctx.accounts.user.key();
    
    // User account should already be initialized
    
    // Validate leverage tier
    let leverage_tier = get_leverage_tier(leverage as u16, size)?;
    
    // Calculate required initial margin
    let required_margin = initial_margin(size, entry_price, leverage)?;
    
    // Check if user has sufficient collateral
    let available_collateral = user_account.total_collateral
        .checked_sub(user_account.locked_collateral)
        .ok_or(ValidationError::MathOverflow)?;
    
    require!(
        available_collateral >= required_margin,
        ValidationError::InsufficientMargin
    );
    
    // Calculate liquidation price
    let liquidation_price = match side {
        Side::Long => liquidation_price_long(
            entry_price,
            leverage,
            leverage_tier.maintenance_margin_rate,
        )?,
        Side::Short => liquidation_price_short(
            entry_price,
            leverage,
            leverage_tier.maintenance_margin_rate,
        )?,
    };
    
    // Initialize position
    position.owner = user_key;
    position.symbol = symbol.clone();
    position.side = side;
    position.size = size;
    position.entry_price = entry_price;
    position.margin = required_margin;
    position.leverage = leverage;
    position.unrealized_pnl = 0;
    position.realized_pnl = 0;
    position.funding_accrued = 0;
    position.liquidation_price = liquidation_price;
    position.last_update = Clock::get()?.unix_timestamp;
    position.bump = ctx.bumps.position;
    
    // Update user account
    user_account.locked_collateral = user_account.locked_collateral
        .checked_add(required_margin)
        .ok_or(ValidationError::MathOverflow)?;
    user_account.position_count = user_account.position_count
        .checked_add(1)
        .ok_or(ValidationError::MathOverflow)?;
    
    // Emit event
    emit!(PositionOpened {
        user: user_key,
        symbol: symbol.clone(),
        side: position.side.clone(),
        size,
        entry_price,
        margin: required_margin,
        leverage,
        liquidation_price,
    });
    
    msg!(
        "Position opened: {} {} {} at {} with leverage {}x",
        if matches!(side, Side::Long) { "LONG" } else { "SHORT" },
        size,
        symbol,
        entry_price,
        leverage
    );
    
    Ok(())
}

#[event]
pub struct PositionOpened {
    pub user: Pubkey,
    pub symbol: String,
    pub side: Side,
    pub size: u64,
    pub entry_price: u64,
    pub margin: u64,
    pub leverage: u8,
    pub liquidation_price: u64,
}
