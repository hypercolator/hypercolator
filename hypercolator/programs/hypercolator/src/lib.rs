use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

pub mod errors;
pub mod state;

#[program]
pub mod hypercolator {
    use super::*;

    /// Initialize a new perpetual market for any SPL token.
    ///
    /// The market is self-funded - all trading fees flow directly into the
    /// insurance fund. No external oracle is required; pricing is derived
    /// from an on-chain AMM TWAP.
    pub fn initialize_market(
        _ctx: Context<InitializeMarket>,
        _params: InitializeMarketParams,
    ) -> Result<()> {
        // TODO Task #11: Market Factory implementation
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Instruction accounts
// ---------------------------------------------------------------------------

#[derive(Accounts)]
pub struct InitializeMarket<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

// ---------------------------------------------------------------------------
// Instruction parameters
// ---------------------------------------------------------------------------

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct InitializeMarketParams {
    /// Initial oracle price in quote-token atomic units per 1 base.
    /// Must satisfy 0 < price <= MAX_ORACLE_PRICE (10^12).
    pub init_oracle_price: u64,

    /// Trading fee in basis points (8 = 0.08% - funds insurance).
    pub trading_fee_bps: u64,

    /// Initial margin requirement in basis points (e.g. 1000 = 10%).
    pub initial_margin_bps: u64,

    /// Maintenance margin requirement in basis points (e.g. 500 = 5%).
    pub maintenance_margin_bps: u64,

    /// Minimum warmup duration in slots (anti-manipulation).
    pub h_min: u64,

    /// Maximum warmup duration in slots.
    pub h_max: u64,
}
