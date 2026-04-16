use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

pub mod errors;
pub mod price_engine;
pub mod state;

use errors::HypercolatorError;
use state::{
    CreatorRecord, MarketConfig, MarketRegistry, MarketTier, TwapState,
    MAX_MARKETS_PER_CREATOR, MAX_REGISTRY_MARKETS, MIN_STAKE_LAMPORTS, TRADING_FEE_BPS,
};

#[program]
pub mod hypercolator {
    use super::*;

    /// Create a permissionless perpetual futures market for an SPL token.
    ///
    /// Requires `stake_amount >= MIN_STAKE_LAMPORTS` and fewer than
    /// `MAX_MARKETS_PER_CREATOR` active markets for the caller.
    /// Market tier (A/B/C) is assigned from the mint address.
    pub fn create_market(ctx: Context<CreateMarket>, stake_amount: u64) -> Result<()> {
        require!(
            stake_amount >= MIN_STAKE_LAMPORTS,
            HypercolatorError::StakeTooLow
        );
        require!(
            ctx.accounts.creator_record.market_count < MAX_MARKETS_PER_CREATOR,
            HypercolatorError::TooManyMarkets
        );
        require!(
            ctx.accounts.market_registry.market_count < MAX_REGISTRY_MARKETS,
            HypercolatorError::RegistryFull
        );

        let mint_key = ctx.accounts.token_mint.key();
        let tier = MarketTier::from_mint(&mint_key);

        anchor_lang::system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.creator.to_account_info(),
                    to: ctx.accounts.market_config.to_account_info(),
                },
            ),
            stake_amount,
        )?;

        let config = &mut ctx.accounts.market_config;
        config.token_mint = mint_key;
        config.creator = ctx.accounts.creator.key();
        config.created_at_slot = Clock::get()?.slot;
        config.tier = tier.to_u8();
        config.max_leverage_x = tier.max_leverage_x();
        config.stake_amount = stake_amount;
        config.insurance_fund = 0;
        config.trading_fee_bps = TRADING_FEE_BPS;
        config.is_active = true;
        config.bump = ctx.bumps.market_config;

        let record = &mut ctx.accounts.creator_record;
        if record.creator == Pubkey::default() {
            record.creator = ctx.accounts.creator.key();
            record.bump = ctx.bumps.creator_record;
        }
        record.market_count += 1;

        let registry = &mut ctx.accounts.market_registry;
        if registry.market_count == 0 {
            registry.bump = ctx.bumps.market_registry;
        }
        registry.markets.push(ctx.accounts.market_config.key());
        registry.market_count += 1;

        emit!(MarketCreated {
            token_mint: mint_key,
            creator: ctx.accounts.creator.key(),
            tier: tier as u8,
            max_leverage_x: tier.max_leverage_x(),
            stake_amount,
        });

        Ok(())
    }

    /// Advance the TWAP accumulator by reading reserve balances from on-chain
    /// SPL Token vault accounts.
    ///
    /// The keeper passes the two AMM vault `TokenAccount`s each slot. On the
    /// first call the vault pubkeys are written into `TwapState` and the
    /// base-vault mint is verified against the market's token mint, binding
    /// this accumulator to that specific pool for all future calls.
    ///
    /// Subsequent calls enforce that the same vault accounts are used, making
    /// vault substitution attacks impossible regardless of keeper behaviour.
    pub fn update_twap(ctx: Context<UpdateTwap>) -> Result<()> {
        let reserve_a = ctx.accounts.vault_base.amount;
        let reserve_b = ctx.accounts.vault_quote.amount;

        require!(
            price_engine::sufficient_liquidity(reserve_a, reserve_b),
            HypercolatorError::InsufficientLiquidity
        );

        let spot_q32 = price_engine::spot_price_q32(reserve_a, reserve_b)
            .ok_or(HypercolatorError::InvalidOraclePrice)?;

        let now_slot = Clock::get()?.slot;
        let market_key = ctx.accounts.market_config.key();
        let vault_base_key = ctx.accounts.vault_base.key();
        let vault_quote_key = ctx.accounts.vault_quote.key();
        let state = &mut ctx.accounts.twap_state;

        if state.last_update_slot == 0 {
            // First observation: only the market creator may bind the pool vaults.
            // This prevents frontrunning by arbitrary keepers who could otherwise
            // permanently lock in attacker-controlled token accounts as price sources.
            require!(
                ctx.accounts.keeper.key() == ctx.accounts.market_config.creator,
                HypercolatorError::UnauthorizedPoolBinding
            );
            // Base vault mint must match the market's token.
            require!(
                ctx.accounts.vault_base.mint == ctx.accounts.market_config.token_mint,
                HypercolatorError::InvalidOraclePrice
            );
            // Vault authority must be off the Ed25519 curve, which means it is a
            // program-derived address rather than a user wallet.  AMM pool vaults
            // are always controlled by PDAs; user-owned token accounts are on-curve.
            require!(
                !ctx.accounts.vault_base.owner.is_on_curve(),
                HypercolatorError::VaultAuthorityNotProgram
            );
            state.market = market_key;
            state.bump = ctx.bumps.twap_state;
            state.base_vault = vault_base_key;
            state.quote_vault = vault_quote_key;
            state.last_spot_q32 = spot_q32;
            state.last_update_slot = now_slot;
            state.window_start_slot = now_slot;
            state.window_start_cumulative = 0;
            state.cumulative_price = 0;
            state.min_observation_slots = price_engine::TWAP_WINDOW_SLOTS;
        } else {
            // Subsequent calls: enforce same vaults (vault substitution guard).
            require!(
                state.base_vault == vault_base_key && state.quote_vault == vault_quote_key,
                HypercolatorError::PoolMismatch
            );
            let elapsed = now_slot.saturating_sub(state.last_update_slot);
            let increment = (state.last_spot_q32 as u128)
                .checked_mul(elapsed as u128)
                .ok_or(HypercolatorError::RiskEngineOverflow)?;
            state.cumulative_price = state
                .cumulative_price
                .checked_add(increment)
                .ok_or(HypercolatorError::RiskEngineOverflow)?;
            state.last_spot_q32 = spot_q32;
            state.last_update_slot = now_slot;

            if now_slot.saturating_sub(state.window_start_slot)
                >= price_engine::TWAP_WINDOW_SLOTS
            {
                state.window_start_slot = now_slot;
                state.window_start_cumulative = state.cumulative_price;
            }
        }

        emit!(TwapUpdated {
            market: market_key,
            spot_q32,
            slot: now_slot,
        });

        Ok(())
    }

    /// Trigger liquidation of an under-collateralised position.
    ///
    /// Reads current pool reserves directly from the bound AMM vault accounts
    /// (verified against pubkeys stored in `TwapState`), then checks:
    ///   1. TWAP window is warmed up.
    ///   2. Pool has sufficient liquidity.
    ///   3. Spot price has not pumped > 20% above TWAP (pump manipulation guard).
    ///
    /// Full position accounting (PnL, insurance fund) is implemented in Task #13.
    pub fn liquidate(ctx: Context<Liquidate>) -> Result<()> {
        let now_slot = Clock::get()?.slot;
        let state = &ctx.accounts.twap_state;

        let twap_q32 = state
            .twap(now_slot)
            .ok_or(HypercolatorError::TwapWindowTooShort)?;

        let reserve_a = ctx.accounts.vault_base.amount;
        let reserve_b = ctx.accounts.vault_quote.amount;

        require!(
            price_engine::sufficient_liquidity(reserve_a, reserve_b),
            HypercolatorError::InsufficientLiquidity
        );

        let spot_q32 = price_engine::spot_price_q32(reserve_a, reserve_b)
            .ok_or(HypercolatorError::InvalidOraclePrice)?;

        require!(
            price_engine::within_deviation(spot_q32, twap_q32),
            HypercolatorError::PriceDeviationTooHigh
        );

        emit!(LiquidationTriggered {
            market: ctx.accounts.market_config.key(),
            twap_q32,
            spot_q32,
            slot: now_slot,
        });

        Ok(())
    }
}

// --- Events ---

#[event]
pub struct MarketCreated {
    pub token_mint: Pubkey,
    pub creator: Pubkey,
    pub tier: u8,
    pub max_leverage_x: u8,
    pub stake_amount: u64,
}

#[event]
pub struct TwapUpdated {
    pub market: Pubkey,
    pub spot_q32: u64,
    pub slot: u64,
}

#[event]
pub struct LiquidationTriggered {
    pub market: Pubkey,
    pub twap_q32: u64,
    pub spot_q32: u64,
    pub slot: u64,
}

// --- Account structs ---

#[derive(Accounts)]
pub struct CreateMarket<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    pub token_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = creator,
        space = MarketConfig::SPACE,
        seeds = [b"market", token_mint.key().as_ref()],
        bump,
    )]
    pub market_config: Account<'info, MarketConfig>,

    #[account(
        init_if_needed,
        payer = creator,
        space = MarketRegistry::space(MAX_REGISTRY_MARKETS),
        seeds = [b"registry"],
        bump,
    )]
    pub market_registry: Account<'info, MarketRegistry>,

    #[account(
        init_if_needed,
        payer = creator,
        space = CreatorRecord::SPACE,
        seeds = [b"creator", creator.key().as_ref()],
        bump,
    )]
    pub creator_record: Account<'info, CreatorRecord>,

    pub system_program: Program<'info, System>,
    /// Validates that token_mint is an SPL token mint; unused in CPI but
    /// retained to keep IDL consistent with the compiled .so.
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct UpdateTwap<'info> {
    #[account(mut)]
    pub keeper: Signer<'info>,

    #[account(
        seeds = [b"market", market_config.token_mint.as_ref()],
        bump = market_config.bump,
    )]
    pub market_config: Account<'info, MarketConfig>,

    #[account(
        init_if_needed,
        payer = keeper,
        space = TwapState::SPACE,
        seeds = [b"twap", market_config.key().as_ref()],
        bump,
    )]
    pub twap_state: Account<'info, TwapState>,

    /// AMM vault holding the base token (mint must equal market_config.token_mint
    /// on first call; enforced via stored pubkey on subsequent calls).
    pub vault_base: Account<'info, TokenAccount>,

    /// AMM vault holding the quote token (e.g., USDC, WSOL).
    pub vault_quote: Account<'info, TokenAccount>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Liquidate<'info> {
    pub keeper: Signer<'info>,

    #[account(
        seeds = [b"market", market_config.token_mint.as_ref()],
        bump = market_config.bump,
    )]
    pub market_config: Account<'info, MarketConfig>,

    #[account(
        seeds = [b"twap", market_config.key().as_ref()],
        bump = twap_state.bump,
    )]
    pub twap_state: Account<'info, TwapState>,

    /// Base token vault — Anchor verifies the address matches twap_state.base_vault.
    #[account(address = twap_state.base_vault)]
    pub vault_base: Account<'info, TokenAccount>,

    /// Quote token vault — Anchor verifies the address matches twap_state.quote_vault.
    #[account(address = twap_state.quote_vault)]
    pub vault_quote: Account<'info, TokenAccount>,
}

// --- Inline unit tests ---

#[cfg(test)]
mod tests {
    use super::*;
    use state::{MarketTier, TwapState, MAX_MARKETS_PER_CREATOR, MIN_STAKE_LAMPORTS, TRADING_FEE_BPS};

    #[test]
    fn tier_unknown_mint_is_c() {
        assert_eq!(MarketTier::from_mint(&Pubkey::new_unique()), MarketTier::C);
    }

    #[test]
    fn tier_usdc_mainnet_is_a() {
        let usdc: Pubkey = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
            .parse()
            .unwrap();
        assert_eq!(MarketTier::from_mint(&usdc), MarketTier::A);
    }

    #[test]
    fn tier_msol_mainnet_is_b() {
        let msol: Pubkey = "mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So"
            .parse()
            .unwrap();
        assert_eq!(MarketTier::from_mint(&msol), MarketTier::B);
    }

    #[test]
    fn tier_max_leverage() {
        assert_eq!(MarketTier::A.max_leverage_x(), 20);
        assert_eq!(MarketTier::B.max_leverage_x(), 10);
        assert_eq!(MarketTier::C.max_leverage_x(), 5);
    }

    #[test]
    fn constants_are_sane() {
        assert_eq!(MIN_STAKE_LAMPORTS, 1_000_000);
        assert_eq!(MAX_MARKETS_PER_CREATOR, 3);
        assert_eq!(TRADING_FEE_BPS, 8);
    }

    #[test]
    fn market_config_space() {
        assert_eq!(MarketConfig::SPACE, 102);
    }

    #[test]
    fn creator_record_space() {
        assert_eq!(CreatorRecord::SPACE, 42);
    }

    #[test]
    fn registry_space_at_capacity() {
        assert_eq!(MarketRegistry::space(MAX_REGISTRY_MARKETS), 8209);
    }

    // --- TwapState::twap() ---

    fn make_twap_state(
        cumulative: u128,
        window_start_cumulative: u128,
        window_start_slot: u64,
        min_slots: u64,
    ) -> TwapState {
        TwapState {
            market: Pubkey::default(),
            bump: 255,
            last_spot_q32: 0,
            cumulative_price: cumulative,
            last_update_slot: 0,
            min_observation_slots: min_slots,
            window_start_cumulative,
            window_start_slot,
            base_vault: Pubkey::default(),
            quote_vault: Pubkey::default(),
        }
    }

    #[test]
    fn twap_window_not_warmed_up() {
        let s = make_twap_state(500, 0, 100, 50);
        assert!(s.twap(140).is_none()); // elapsed = 40 < min_slots 50
    }

    #[test]
    fn twap_computes_correctly() {
        // window [0, 100]: cumulative = 1_000_000, delta = 1_000_000, twap = 10_000
        let s = make_twap_state(1_000_000, 0, 0, 50);
        assert_eq!(s.twap(100), Some(10_000));
    }

    #[test]
    fn twap_after_window_slide() {
        // window [1000, 1100]: delta = 6_100_000 - 5_000_000, twap = 11_000
        let s = make_twap_state(6_100_000, 5_000_000, 1000, 50);
        assert_eq!(s.twap(1100), Some(11_000));
    }

    #[test]
    fn twap_state_space() {
        // 8 + 32 + 1 + 8 + 16 + 8 + 8 + 16 + 8 + 32 + 32 = 169
        assert_eq!(TwapState::SPACE, 169);
    }
}
