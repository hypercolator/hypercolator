use anchor_lang::prelude::*;

// ---------------------------------------------------------------------------
// Protocol constants
// ---------------------------------------------------------------------------

/// Minimum lamports a creator must bond to open a new market (0.001 SOL).
/// Returned to the creator when the market is closed (Task #13).
pub const MIN_STAKE_LAMPORTS: u64 = 1_000_000;

/// Maximum number of active markets any single creator wallet may open.
pub const MAX_MARKETS_PER_CREATOR: u8 = 3;

/// Insurance fee charged on every trade, in basis points (0.08%).
/// All collected fees flow directly into the market's on-chain insurance fund.
pub const TRADING_FEE_BPS: u16 = 8;

/// Maximum number of markets tracked in the global registry PDA.
/// 256 entries → space = 8 + 1 + 4 + 4 + 32*256 = 8209 bytes, safely under
/// the 10240-byte per-instruction CPI data-increase limit imposed by Solana
/// 1.14+ (a larger registry can be migrated via a grow-registry instruction
/// in a later task).
pub const MAX_REGISTRY_MARKETS: u32 = 256;

// ---------------------------------------------------------------------------
// Market tier
// ---------------------------------------------------------------------------

/// Risk tier for a perpetual market, determined at creation from the base mint.
///
/// | Tier | Token class                   | Max leverage | Raw u8 |
/// |------|-------------------------------|--------------|--------|
/// | A    | Major / blue-chip             | 20x          | 0      |
/// | B    | Other established SPL tokens  | 10x          | 1      |
/// | C    | Unknown / pump.fun / new      | 5x           | 2      |
///
/// Stored as `u8` in `MarketConfig` to avoid borsh version ambiguity with
/// enum derive macros.  Use `MarketTier::from_u8` / `to_u8` to convert.
///
/// Tier can be upgraded via governance (Task #13) but never downgraded.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarketTier {
    A = 0,
    B = 1,
    C = 2,
}

impl MarketTier {
    /// Deserialise from the raw `u8` stored in `MarketConfig.tier`.
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => MarketTier::A,
            1 => MarketTier::B,
            _ => MarketTier::C,
        }
    }

    /// Serialise to the raw `u8` stored in `MarketConfig.tier`.
    pub fn to_u8(self) -> u8 {
        self as u8
    }

    /// Leverage cap in whole multiples (e.g. 5 means 5x).
    pub fn max_leverage_x(self) -> u8 {
        match self {
            MarketTier::A => 20,
            MarketTier::B => 10,
            MarketTier::C => 5,
        }
    }

    /// Classify a token mint into a tier.
    ///
    /// Looks the mint up in hard-coded allow-lists using the base-58 string
    /// representation.  Any mint not on the lists falls into Tier C, which is
    /// the default for permissionless markets (pump.fun tokens, novel assets).
    ///
    /// The lists cover mainnet addresses only.  On devnet/localnet all markets
    /// will be Tier C unless a test overrides the tier directly.
    pub fn from_mint(mint: &Pubkey) -> Self {
        let s = mint.to_string();
        match s.as_str() {
            // ---- Tier A: major / blue-chip tokens (mainnet) ----
            "So11111111111111111111111111111111111111112"      // Wrapped SOL
            | "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v" // USDC
            | "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB"  // USDT
            | "9n4nbM75f5Ui33ZbPYXn59EwSgE8CGsHtAeTH5YFeJ9E"  // Wrapped BTC
            | "7vfCXTUXx5WJV5JADk17DUJ4ksgau7utNKj4b963voxs"  // Wrapped ETH (Wormhole)
            => MarketTier::A,

            // ---- Tier B: established SPL tokens (mainnet) ----
            "mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So"    // mSOL (Marinade)
            | "7dHbWXmci3dT8UFYWYZweBLXgycu7Y3iL6trKn1Y68YB"  // stSOL (Lido)
            | "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R"  // RAY (Raydium)
            | "SRMuApVNdxXokk5GT7XD5cUUgXMBCoAz2LHeuAoKWRt"   // SRM (Serum)
            | "orcaEKTdK7LKz57vaAYr9QeNsVEPfiu6QeMU1kektZE"   // ORCA
            => MarketTier::B,

            // ---- Tier C: everything else (pump.fun, unknown, new tokens) ----
            _ => MarketTier::C,
        }
    }
}

// ---------------------------------------------------------------------------
// MarketConfig  —  seeds = [b"market", token_mint]
// ---------------------------------------------------------------------------

/// Per-market configuration stored in a PDA keyed by the base token mint.
///
/// One PDA per mint enforces uniqueness: there can be at most one perpetual
/// market per token on Hypercolator.
///
/// Created by `create_market`, read by trading instructions (Task #12+).
#[account]
#[derive(Debug)]
pub struct MarketConfig {
    /// The SPL token mint this market trades (base token).
    pub token_mint: Pubkey,

    /// Wallet that created this market and bonded the stake.
    pub creator: Pubkey,

    /// Solana slot at which the market was created.
    pub created_at_slot: u64,

    /// Risk tier encoded as u8 (0=A, 1=B, 2=C).  Use `MarketTier::from_u8` to decode.
    pub tier: u8,

    /// Max leverage this market allows (whole multiples, e.g. 5 → 5x).
    pub max_leverage_x: u8,

    /// Creator's bonded stake in lamports, locked until market closure.
    pub stake_amount: u64,

    /// On-chain insurance fund in lamports accumulated from TRADING_FEE_BPS.
    pub insurance_fund: u64,

    /// Fee per trade in basis points (always TRADING_FEE_BPS = 8 = 0.08%).
    pub trading_fee_bps: u16,

    /// Whether the market is currently accepting new positions.
    pub is_active: bool,

    /// PDA bump seed — stored for deterministic re-derivation.
    pub bump: u8,
}

impl MarketConfig {
    /// Total byte size of the serialized account, including Anchor discriminator.
    pub const SPACE: usize = 8   // Anchor 8-byte discriminator
        + 32  // token_mint
        + 32  // creator
        + 8   // created_at_slot
        + 1   // tier (repr u8)
        + 1   // max_leverage_x
        + 8   // stake_amount
        + 8   // insurance_fund
        + 2   // trading_fee_bps
        + 1   // is_active
        + 1; // bump
}

// ---------------------------------------------------------------------------
// MarketRegistry  —  seeds = [b"registry"]
// ---------------------------------------------------------------------------

/// Global singleton PDA listing all MarketConfig addresses.
///
/// Initialised on the first `create_market` call.  Subsequent calls append
/// to `markets`.  Bounded by `MAX_REGISTRY_MARKETS` (256).
#[account]
#[derive(Debug)]
pub struct MarketRegistry {
    /// PDA bump seed.
    pub bump: u8,

    /// Total markets registered (monotonically increasing, never decremented).
    pub market_count: u32,

    /// Ordered list of MarketConfig PDA public keys.
    pub markets: Vec<Pubkey>,
}

impl MarketRegistry {
    /// Account byte size for the given market capacity.
    pub fn space(capacity: u32) -> usize {
        8               // discriminator
        + 1             // bump
        + 4             // market_count
        + 4             // Vec length prefix
        + 32 * capacity as usize // markets[capacity]
    }
}

// ---------------------------------------------------------------------------
// CreatorRecord  —  seeds = [b"creator", creator]
// ---------------------------------------------------------------------------

/// Per-wallet record tracking how many markets a creator has opened.
///
/// Capped at `MAX_MARKETS_PER_CREATOR` (3) to prevent market spam.
/// The count is incremented on creation and decremented on closure (Task #13).
#[account]
#[derive(Debug)]
pub struct CreatorRecord {
    /// The creator wallet this record belongs to.
    pub creator: Pubkey,

    /// Number of currently active markets opened by this wallet.
    pub market_count: u8,

    /// PDA bump seed.
    pub bump: u8,
}

impl CreatorRecord {
    /// Total byte size including discriminator.
    pub const SPACE: usize = 8  // discriminator
        + 32  // creator
        + 1   // market_count
        + 1; // bump
}

// ---------------------------------------------------------------------------
// MarketState / TwapState  —  reserved for Task #12 (price engine)
// ---------------------------------------------------------------------------

/// Thin Anchor wrapper around Percolator's RiskEngine (Task #12).
#[account]
#[derive(Debug)]
pub struct MarketState {
    pub authority: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub vault: Pubkey,
    pub initialized: bool,
    pub bump: u8,
    pub created_at_slot: u64,
    /// Serialised RiskEngine bytes (populated in Task #12).
    pub engine_data: Vec<u8>,
}

impl MarketState {
    pub const BASE_SIZE: usize = 8
        + 32  // authority
        + 32  // base_mint
        + 32  // quote_mint
        + 32  // vault
        + 1   // initialized
        + 1   // bump
        + 8   // created_at_slot
        + 4; // Vec length prefix

    pub fn space(engine_data_size: usize) -> usize {
        Self::BASE_SIZE + engine_data_size
    }
}

/// AMM TWAP accumulator for a market, PDA seeds = [b"twap", market_config].
///
/// The keeper calls `update_twap` every slot, passing current AMM reserves.
/// Price is stored in Q32 fixed-point (units of quote-per-base, scaled 2^32).
/// TWAP is computed over the most recent `min_observation_slots` window.
#[account]
#[derive(Debug)]
pub struct TwapState {
    /// The MarketConfig PDA this accumulator belongs to.
    pub market: Pubkey,
    /// PDA bump seed.
    pub bump: u8,
    /// Last observed spot price in Q32 (price = reserve_b / reserve_a * 2^32).
    pub last_spot_q32: u64,
    /// Cumulative sum of (spot_q32 * elapsed_slots) since initialisation.
    pub cumulative_price: u128,
    /// Solana slot of the most recent `update_twap` call.
    pub last_update_slot: u64,
    /// Minimum window length (slots) required before TWAP is considered valid.
    pub min_observation_slots: u64,
    /// Cumulative price snapshot at the start of the current TWAP window.
    pub window_start_cumulative: u128,
    /// Slot at which the current TWAP window began.
    pub window_start_slot: u64,
}

impl TwapState {
    pub const SPACE: usize = 8
        + 32  // market
        + 1   // bump
        + 8   // last_spot_q32
        + 16  // cumulative_price
        + 8   // last_update_slot
        + 8   // min_observation_slots
        + 16  // window_start_cumulative
        + 8;  // window_start_slot

    /// Compute the time-weighted average price over the current window.
    ///
    /// Returns `None` if fewer than `min_observation_slots` have elapsed
    /// since the window started (TWAP not yet warmed up).
    pub fn twap(&self, now_slot: u64) -> Option<u64> {
        let elapsed = now_slot.saturating_sub(self.window_start_slot);
        if elapsed < self.min_observation_slots {
            return None;
        }
        let delta = self.cumulative_price.saturating_sub(self.window_start_cumulative);
        let twap = delta.checked_div(elapsed as u128)?;
        u64::try_from(twap).ok()
    }
}
