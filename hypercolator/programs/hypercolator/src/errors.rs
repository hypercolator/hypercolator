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

    #[msg("Market registry is full (maximum 256 markets reached)")]
    RegistryFull,

    // ---- Oracle / Price Engine (Task #12) ----
    #[msg("Invalid oracle price: reserve_a is zero or result overflows")]
    InvalidOraclePrice,

    #[msg("TWAP observation window too short — price manipulation risk")]
    TwapWindowTooShort,

    #[msg("Spot price deviates from TWAP by more than 20%")]
    PriceDeviationTooHigh,

    #[msg("AMM pool liquidity is below the minimum required threshold")]
    InsufficientLiquidity,

    #[msg("Vault accounts do not match the pool bound to this TWAP accumulator")]
    PoolMismatch,

    #[msg("Only the market creator may bind the AMM pool vaults on first update")]
    UnauthorizedPoolBinding,

    #[msg("Vault authority must be a program-derived address, not a user wallet")]
    VaultAuthorityNotProgram,

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
