use anchor_lang::prelude::*;

#[error_code]
pub enum ValidationError {
    #[msg("Invalid leverage specified")]
    InvalidLeverage,
    #[msg("Insufficient margin to open position")]
    InsufficientMargin,
    #[msg("Position size exceeds maximum allowed for leverage tier")]
    PositionTooLarge,
    #[msg("Math operation resulted in overflow")]
    MathOverflow,
    #[msg("Unauthorized access to account")]
    Unauthorized,
    #[msg("Position account not found")]
    PositionNotFound,
    #[msg("User account not initialized")]
    UserNotInitialized,
    #[msg("Leverage exceeds maximum allowed for position size")]
    LeverageExceeded,
    #[msg("Cannot remove margin: would make position unsafe")]
    UnsafeMarginRemoval,
    #[msg("Invalid position modification parameters")]
    InvalidModification,
    #[msg("Position size cannot be zero")]
    ZeroPositionSize,
    #[msg("Cannot close position with non-zero size")]
    NonZeroPositionSize,
}
