use anchor_lang::prelude::*;
use crate::state::*;

#[derive(Accounts)]
pub struct InitUser<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    
    #[account(
        init,
        payer = user,
        space = UserAccount::LEN,
        seeds = [USER_ACCOUNT_SEED, user.key().as_ref()],
        bump
    )]
    pub user_account: Account<'info, UserAccount>,
    
    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<InitUser>,
    initial_collateral: u64,
) -> Result<()> {
    let user_account = &mut ctx.accounts.user_account;
    let user_key = ctx.accounts.user.key();
    
    user_account.owner = user_key;
    user_account.total_collateral = initial_collateral;
    user_account.locked_collateral = 0;
    user_account.total_pnl = 0;
    user_account.position_count = 0;
    user_account.bump = ctx.bumps.user_account;
    
    msg!("User account initialized for {} with collateral {}", user_key, initial_collateral);
    
    Ok(())
}
