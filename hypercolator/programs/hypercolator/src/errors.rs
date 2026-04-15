use anchor_lang::prelude::*;

#[error_code]
pub enum HypercolatorError {
    // ---- Market Factory (Task #11) ----
    #[msg("Stake amount is below the minimum required to create a market")]
    StakeTooLow,

    #[msg("Creator wallet has reached the maximum number of active markets (3)")]
    TooManyMarkets,

    #[msg("A market already exists for this token mint")]
    MarketAlreadyExists,

    #[msg("Market registry is full (maximum 1024 markets reached)")]
    RegistryFull,

    // ---- Oracle / Price Engine (Task #12) ----
    #[msg("Invalid oracle price: must be in range (0, MAX_ORACLE_PRICE]")]
    InvalidOraclePrice,

    #[msg("TWAP observation window too short - price manipulation risk")]
    TwapWindowTooShort,

    // ---- Margin / Risk Engine ----
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

    #[msg("Side is in DrainOnly or ResetPending mode - no new OI allowed")]
    SideBlocked,

    #[msg("PnL not yet warmed up - cannot withdraw")]
    PnlNotWarmedUp,

    #[msg("Risk engine arithmetic overflow")]
    RiskEngineOverflow,

    #[msg("Insurance fund below minimum floor")]
    InsuranceBelowFloor,
}
