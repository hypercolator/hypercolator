use anchor_lang::prelude::*;

/// Hypercolator market state account.
///
/// This is a thin Anchor wrapper around the Percolator RiskEngine.
/// The actual risk math lives in the `percolator` crate.
///
/// Size budget: the Percolator RiskEngine with MAX_ACCOUNTS = 4096 is
/// approximately 4096 * ~500 bytes/account = ~2MB. In practice Solana
/// accounts are limited to 10MB so this fits, but deployment will use
/// a smaller MAX_ACCOUNTS for cost efficiency.
#[account]
#[derive(Debug)]
pub struct MarketState {
    /// Authority that can resolve the market or update parameters.
    pub authority: Pubkey,

    /// The base token mint (e.g. the pump.fun token being traded).
    pub base_mint: Pubkey,

    /// The quote token mint (e.g. USDC).
    pub quote_mint: Pubkey,

    /// The SPL token vault holding all quote tokens.
    pub vault: Pubkey,

    /// Whether the market is initialized.
    pub initialized: bool,

    /// Bump seed for the market PDA.
    pub bump: u8,

    /// Slot at which this market was created.
    pub created_at_slot: u64,

    /// Raw serialized RiskEngine state. Stored as bytes to allow
    /// zero-copy access patterns on Solana.
    ///
    /// The actual size depends on MAX_ACCOUNTS compile-time constant.
    /// This field is intentionally last for future size flexibility.
    pub engine_data: Vec<u8>,
}

impl MarketState {
    /// Anchor discriminator (8) + fixed fields
    pub const BASE_SIZE: usize = 8
        + 32  // authority
        + 32  // base_mint
        + 32  // quote_mint
        + 32  // vault
        + 1   // initialized
        + 1   // bump
        + 8   // created_at_slot
        + 4;  // Vec<u8> length prefix

    /// Space for a market with the given engine data size.
    pub fn space(engine_data_size: usize) -> usize {
        Self::BASE_SIZE + engine_data_size
    }
}

/// AMM observation for TWAP calculation.
///
/// Each AMM interaction writes a price observation. The TWAP is derived
/// from the cumulative price over the observation window. This eliminates
/// the need for any external oracle - price discovery is entirely on-chain.
#[account]
#[derive(Debug)]
pub struct TwapState {
    /// The market this TWAP belongs to.
    pub market: Pubkey,

    /// Bump seed.
    pub bump: u8,

    /// Current AMM price in quote-token atomic units per 1 base.
    /// Stored as sqrt(price) * 2^32 for Uniswap v3-style math.
    pub sqrt_price_x32: u64,

    /// Cumulative price sum (price * slots elapsed) for TWAP.
    pub cumulative_price: u128,

    /// Slot of last AMM interaction.
    pub last_update_slot: u64,

    /// Minimum observation window required for a valid TWAP (slots).
    pub min_observation_slots: u64,

    /// Snapshot of cumulative_price at the start of the observation window.
    pub window_start_cumulative: u128,

    /// Slot at which the observation window started.
    pub window_start_slot: u64,
}

impl TwapState {
    pub const SPACE: usize = 8   // discriminator
        + 32  // market
        + 1   // bump
        + 8   // sqrt_price_x32
        + 16  // cumulative_price
        + 8   // last_update_slot
        + 8   // min_observation_slots
        + 16  // window_start_cumulative
        + 8;  // window_start_slot

    /// Compute the TWAP over the current observation window.
    ///
    /// Returns None if the window has not been open long enough.
    pub fn twap(&self, now_slot: u64) -> Option<u64> {
        let elapsed = now_slot.saturating_sub(self.window_start_slot);
        if elapsed < self.min_observation_slots {
            return None;
        }
        let delta_cumulative = self
            .cumulative_price
            .saturating_sub(self.window_start_cumulative);
        // TWAP = sum(price * slots) / slots
        let twap = delta_cumulative.checked_div(elapsed as u128)?;
        // Clamp to u64 range
        if twap > u64::MAX as u128 {
            return None;
        }
        Some(twap as u64)
    }
}
