use anchor_lang::prelude::*;

#[error_code]
pub enum HypercolatorError {
    #[msg("Invalid oracle price: must be in range (0, MAX_ORACLE_PRICE]")]
    InvalidOraclePrice,

    #[msg("Invalid margin configuration: maintenance must be <= initial")]
    InvalidMarginConfig,

    #[msg("Insufficient collateral for this operation")]
    InsufficientCollateral,

    #[msg("Market is not in Live mode")]
    MarketNotLive,

    #[msg("Market is not in Resolved mode")]
    MarketNotResolved,

    #[msg("Account is not liquidatable")]
    NotLiquidatable,

    #[msg("Trade size exceeds maximum allowed")]
    TradeSizeExceeded,

    #[msg("Open interest limit reached")]
    OpenInterestLimitReached,

    #[msg("Warmup horizon out of allowed range")]
    InvalidWarmupHorizon,

    #[msg("TWAP observation window too short - price manipulation risk")]
    TwapWindowTooShort,

    #[msg("Side is in DrainOnly or ResetPending mode - no new OI allowed")]
    SideBlocked,

    #[msg("PnL not yet warmed up - cannot withdraw")]
    PnlNotWarmedUp,

    #[msg("Risk engine arithmetic overflow")]
    RiskEngineOverflow,

    #[msg("Insurance fund below minimum floor")]
    InsuranceBelowFloor,
}
