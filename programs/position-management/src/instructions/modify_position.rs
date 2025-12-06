use anchor_lang::prelude::*;
use crate::state::*;
use crate::calculations::*;
use crate::error::ValidationError;

#[derive(Accounts)]
#[instruction(symbol: String)]
pub struct ModifyPosition<'info> {
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
        seeds = [POSITION_SEED, user.key().as_ref(), symbol.as_bytes()],
        bump = position.bump,
        constraint = position.owner == user.key() @ ValidationError::Unauthorized
    )]
    pub position: Account<'info, Position>,
}

pub fn handler(
    ctx: Context<ModifyPosition>,
    _symbol: String,
    modification_type: ModificationType,
    amount: u64,
    new_entry_price: Option<u64>,
) -> Result<()> {
    let user_account = &mut ctx.accounts.user_account;
    let position = &mut ctx.accounts.position;
    
    match modification_type {
        ModificationType::IncreaseSize => {
            increase_position_size(position, user_account, amount, new_entry_price)?;
        }
        ModificationType::DecreaseSize => {
            decrease_position_size(position, user_account, amount)?;
        }
        ModificationType::AddMargin => {
            add_margin(position, user_account, amount)?;
        }
        ModificationType::RemoveMargin => {
            remove_margin(position, user_account, amount)?;
        }
    }
    
    // Update timestamp
    position.last_update = Clock::get()?.unix_timestamp;
    
    // Emit event
    emit!(PositionModified {
        user: position.owner,
        symbol: position.symbol.clone(),
        modification_type,
        amount,
        new_size: position.size,
        new_margin: position.margin,
        new_entry_price: position.entry_price,
        new_liquidation_price: position.liquidation_price,
    });
    
    Ok(())
}

fn increase_position_size(
    position: &mut Position,
    user_account: &mut UserAccount,
    size_increase: u64,
    new_entry_price: Option<u64>,
) -> Result<()> {
    require!(size_increase > 0, ValidationError::ZeroPositionSize);
    
    let entry_price = new_entry_price.ok_or(ValidationError::InvalidModification)?;
    require!(entry_price > 0, ValidationError::MathOverflow);
    
    let new_total_size = position.size
        .checked_add(size_increase)
        .ok_or(ValidationError::MathOverflow)?;
    
    // Validate leverage tier for new size
    let _tier = get_leverage_tier(position.leverage as u16, new_total_size)?;
    
    // Calculate new weighted average entry price
    let new_weighted_entry_price = weighted_avg_entry_price(
        position.size,
        position.entry_price,
        size_increase,
        entry_price,
    )?;
    
    // Calculate additional margin required
    let additional_margin = initial_margin(size_increase, entry_price, position.leverage)?;
    
    // Check available collateral
    let available_collateral = user_account.total_collateral
        .checked_sub(user_account.locked_collateral)
        .ok_or(ValidationError::MathOverflow)?;
    
    require!(
        available_collateral >= additional_margin,
        ValidationError::InsufficientMargin
    );
    
    // Update position
    position.size = new_total_size;
    position.entry_price = new_weighted_entry_price;
    position.margin = position.margin
        .checked_add(additional_margin)
        .ok_or(ValidationError::MathOverflow)?;
    
    // Recalculate liquidation price
    let leverage_tier = get_leverage_tier(position.leverage as u16, position.size)?;
    position.liquidation_price = match position.side {
        Side::Long => liquidation_price_long(
            position.entry_price,
            position.leverage,
            leverage_tier.maintenance_margin_rate,
        )?,
        Side::Short => liquidation_price_short(
            position.entry_price,
            position.leverage,
            leverage_tier.maintenance_margin_rate,
        )?,
    };
    
    // Update user account
    user_account.locked_collateral = user_account.locked_collateral
        .checked_add(additional_margin)
        .ok_or(ValidationError::MathOverflow)?;
    
    Ok(())
}

fn decrease_position_size(
    position: &mut Position,
    user_account: &mut UserAccount,
    size_decrease: u64,
) -> Result<()> {
    require!(size_decrease > 0, ValidationError::ZeroPositionSize);
    require!(size_decrease < position.size, ValidationError::InvalidModification);
    
    let new_size = position.size
        .checked_sub(size_decrease)
        .ok_or(ValidationError::MathOverflow)?;
    
    // Calculate margin to unlock (proportional)
    let margin_to_unlock = position.margin
        .checked_mul(size_decrease)
        .ok_or(ValidationError::MathOverflow)?
        .checked_div(position.size)
        .ok_or(ValidationError::MathOverflow)?;
    
    // Calculate realized PnL for closed portion
    let current_price = position.entry_price; // In real implementation, this would be mark price
    let is_long = matches!(position.side, Side::Long);
    let partial_pnl = unrealized_pnl(is_long, size_decrease, current_price, position.entry_price)?;
    
    // Update position
    position.size = new_size;
    position.margin = position.margin
        .checked_sub(margin_to_unlock)
        .ok_or(ValidationError::MathOverflow)?;
    position.realized_pnl = position.realized_pnl
        .checked_add(partial_pnl)
        .ok_or(ValidationError::MathOverflow)?;
    
    // Recalculate liquidation price
    let leverage_tier = get_leverage_tier(position.leverage as u16, position.size)?;
    position.liquidation_price = match position.side {
        Side::Long => liquidation_price_long(
            position.entry_price,
            position.leverage,
            leverage_tier.maintenance_margin_rate,
        )?,
        Side::Short => liquidation_price_short(
            position.entry_price,
            position.leverage,
            leverage_tier.maintenance_margin_rate,
        )?,
    };
    
    // Update user account
    user_account.locked_collateral = user_account.locked_collateral
        .checked_sub(margin_to_unlock)
        .ok_or(ValidationError::MathOverflow)?;
    user_account.total_pnl = user_account.total_pnl
        .checked_add(partial_pnl)
        .ok_or(ValidationError::MathOverflow)?;
    
    Ok(())
}

fn add_margin(
    position: &mut Position,
    user_account: &mut UserAccount,
    margin_amount: u64,
) -> Result<()> {
    require!(margin_amount > 0, ValidationError::InvalidModification);
    
    // Check available collateral
    let available_collateral = user_account.total_collateral
        .checked_sub(user_account.locked_collateral)
        .ok_or(ValidationError::MathOverflow)?;
    
    require!(
        available_collateral >= margin_amount,
        ValidationError::InsufficientMargin
    );
    
    // Update position and user account
    position.margin = position.margin
        .checked_add(margin_amount)
        .ok_or(ValidationError::MathOverflow)?;
    
    user_account.locked_collateral = user_account.locked_collateral
        .checked_add(margin_amount)
        .ok_or(ValidationError::MathOverflow)?;
    
    // Recalculate liquidation price with new margin
    let leverage_tier = get_leverage_tier(position.leverage as u16, position.size)?;
    position.liquidation_price = match position.side {
        Side::Long => liquidation_price_long(
            position.entry_price,
            position.leverage,
            leverage_tier.maintenance_margin_rate,
        )?,
        Side::Short => liquidation_price_short(
            position.entry_price,
            position.leverage,
            leverage_tier.maintenance_margin_rate,
        )?,
    };
    
    Ok(())
}

fn remove_margin(
    position: &mut Position,
    user_account: &mut UserAccount,
    margin_amount: u64,
) -> Result<()> {
    require!(margin_amount > 0, ValidationError::InvalidModification);
    require!(margin_amount < position.margin, ValidationError::UnsafeMarginRemoval);
    
    let remaining_margin = position.margin
        .checked_sub(margin_amount)
        .ok_or(ValidationError::MathOverflow)?;
    
    // Calculate minimum required margin
    let leverage_tier = get_leverage_tier(position.leverage as u16, position.size)?;
    let position_val = position_value(position.size, position.entry_price)?;
    let min_margin = maintenance_margin(position_val, leverage_tier.maintenance_margin_rate)?;
    
    // Ensure remaining margin is above minimum requirement
    require!(
        remaining_margin >= min_margin,
        ValidationError::UnsafeMarginRemoval
    );
    
    // Update position and user account
    position.margin = remaining_margin;
    user_account.locked_collateral = user_account.locked_collateral
        .checked_sub(margin_amount)
        .ok_or(ValidationError::MathOverflow)?;
    
    // Recalculate liquidation price
    position.liquidation_price = match position.side {
        Side::Long => liquidation_price_long(
            position.entry_price,
            position.leverage,
            leverage_tier.maintenance_margin_rate,
        )?,
        Side::Short => liquidation_price_short(
            position.entry_price,
            position.leverage,
            leverage_tier.maintenance_margin_rate,
        )?,
    };
    
    Ok(())
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub enum ModificationType {
    IncreaseSize,
    DecreaseSize,
    AddMargin,
    RemoveMargin,
}

#[event]
pub struct PositionModified {
    pub user: Pubkey,
    pub symbol: String,
    pub modification_type: ModificationType,
    pub amount: u64,
    pub new_size: u64,
    pub new_margin: u64,
    pub new_entry_price: u64,
    pub new_liquidation_price: u64,
}
