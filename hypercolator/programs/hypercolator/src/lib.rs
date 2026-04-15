use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token};

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

pub mod errors;
pub mod state;

use errors::HypercolatorError;
use state::{
    CreatorRecord, MarketConfig, MarketRegistry, MarketTier, MAX_MARKETS_PER_CREATOR,
    MAX_REGISTRY_MARKETS, MIN_STAKE_LAMPORTS, TRADING_FEE_BPS,
};

#[program]
pub mod hypercolator {
    use super::*;

    /// Create a permissionless perpetual futures market for any SPL token.
    ///
    /// Any wallet may call this instruction to launch a market, subject to:
    ///   - Bonding at least `MIN_STAKE_LAMPORTS` lamports as a spam-prevention
    ///     bond (returned when the market is closed, Task #13).
    ///   - Holding fewer than `MAX_MARKETS_PER_CREATOR` (3) active markets.
    ///
    /// On success:
    ///   - A `MarketConfig` PDA is initialised for the token mint.
    ///   - The mint is appended to the global `MarketRegistry`.
    ///   - The creator's `CreatorRecord` market count is incremented.
    ///   - `stake_amount` lamports are transferred from the creator to the
    ///     `MarketConfig` PDA as a locked bond.
    ///
    /// The market's tier (A / B / C) is assigned automatically based on the
    /// mint address and determines the maximum allowable leverage.
    pub fn create_market(ctx: Context<CreateMarket>, stake_amount: u64) -> Result<()> {
        // ---- Validation ----
        require!(
            stake_amount >= MIN_STAKE_LAMPORTS,
            HypercolatorError::StakeTooLow
        );

        let creator_count = ctx.accounts.creator_record.market_count;
        require!(
            creator_count < MAX_MARKETS_PER_CREATOR,
            HypercolatorError::TooManyMarkets
        );

        let registry_count = ctx.accounts.market_registry.market_count;
        require!(
            registry_count < MAX_REGISTRY_MARKETS,
            HypercolatorError::RegistryFull
        );

        // ---- Tier assignment ----
        let mint_key = ctx.accounts.token_mint.key();
        let tier = MarketTier::from_mint(&mint_key);

        // ---- Transfer stake bond from creator to market PDA ----
        // The stake sits as extra lamports on the MarketConfig account.
        // The account already holds rent-exempt lamports from `init`;
        // we add the bond on top so close_market can return them separately.
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

        // ---- Initialise MarketConfig ----
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

        // ---- Initialise or update CreatorRecord ----
        let record = &mut ctx.accounts.creator_record;
        if record.creator == Pubkey::default() {
            record.creator = ctx.accounts.creator.key();
            record.bump = ctx.bumps.creator_record;
        }
        record.market_count = record.market_count.saturating_add(1);

        // ---- Append to MarketRegistry ----
        let registry = &mut ctx.accounts.market_registry;
        if registry.market_count == 0 {
            registry.bump = ctx.bumps.market_registry;
        }
        registry.markets.push(ctx.accounts.market_config.key());
        registry.market_count = registry.market_count.saturating_add(1);

        emit!(MarketCreated {
            token_mint: mint_key,
            creator: ctx.accounts.creator.key(),
            tier: tier as u8,
            max_leverage_x: tier.max_leverage_x(),
            stake_amount,
        });

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Events
// ---------------------------------------------------------------------------

#[event]
pub struct MarketCreated {
    pub token_mint: Pubkey,
    pub creator: Pubkey,
    pub tier: u8,
    pub max_leverage_x: u8,
    pub stake_amount: u64,
}

// ---------------------------------------------------------------------------
// Account constraints for create_market
// ---------------------------------------------------------------------------

#[derive(Accounts)]
pub struct CreateMarket<'info> {
    /// The wallet creating (and bonding stake for) this market.
    #[account(mut)]
    pub creator: Signer<'info>,

    /// The SPL token mint being listed for perpetual trading.
    /// Verified to be an initialised mint by the Anchor `Account<Mint>` check.
    pub token_mint: Account<'info, Mint>,

    /// Market configuration PDA — one per token mint.
    /// `init` enforces uniqueness: a second attempt for the same mint reverts.
    #[account(
        init,
        payer = creator,
        space = MarketConfig::SPACE,
        seeds = [b"market", token_mint.key().as_ref()],
        bump,
    )]
    pub market_config: Account<'info, MarketConfig>,

    /// Global registry of all market PDAs.
    /// `init_if_needed` initialises it on the very first market creation.
    #[account(
        init_if_needed,
        payer = creator,
        space = MarketRegistry::space(MAX_REGISTRY_MARKETS),
        seeds = [b"registry"],
        bump,
    )]
    pub market_registry: Account<'info, MarketRegistry>,

    /// Per-creator record — tracks how many markets this wallet has opened.
    /// `init_if_needed` initialises on the creator's first market.
    #[account(
        init_if_needed,
        payer = creator,
        space = CreatorRecord::SPACE,
        seeds = [b"creator", creator.key().as_ref()],
        bump,
    )]
    pub creator_record: Account<'info, CreatorRecord>,

    pub system_program: Program<'info, System>,
    /// Token program — required to verify the token_mint account type.
    pub token_program: Program<'info, Token>,
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use state::{MarketTier, MAX_MARKETS_PER_CREATOR, MIN_STAKE_LAMPORTS, TRADING_FEE_BPS};

    // ---- Tier assignment ----

    #[test]
    fn tier_unknown_mint_is_c() {
        // Any random pubkey (pump.fun token, new token, devnet mint) → Tier C.
        let mint = Pubkey::new_unique();
        assert_eq!(MarketTier::from_mint(&mint), MarketTier::C);
    }

    #[test]
    fn tier_usdc_mainnet_is_a() {
        // USDC mainnet mint: EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v
        let usdc: Pubkey = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
            .parse()
            .unwrap();
        assert_eq!(MarketTier::from_mint(&usdc), MarketTier::A);
    }

    #[test]
    fn tier_msol_mainnet_is_b() {
        // mSOL (Marinade): mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So
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

    // ---- Stake validation ----

    /// Helper replicating the create_market stake check (pure function).
    fn validate_stake(amount: u64) -> bool {
        amount >= MIN_STAKE_LAMPORTS
    }

    #[test]
    fn stake_happy_path() {
        // Exactly the minimum is accepted.
        assert!(validate_stake(MIN_STAKE_LAMPORTS));
        // More than the minimum is also accepted.
        assert!(validate_stake(MIN_STAKE_LAMPORTS * 10));
    }

    #[test]
    fn stake_too_low_rejected() {
        // Zero stake is rejected.
        assert!(!validate_stake(0));
        // One lamport below minimum is rejected.
        assert!(!validate_stake(MIN_STAKE_LAMPORTS - 1));
    }

    // ---- Creator market-count validation ----

    /// Helper replicating the market-count check.
    fn can_create_market(current_count: u8) -> bool {
        current_count < MAX_MARKETS_PER_CREATOR
    }

    #[test]
    fn creator_wallet_limit_happy_path() {
        // A fresh wallet (count 0) can create.
        assert!(can_create_market(0));
        // A wallet with 2 markets can still create one more.
        assert!(can_create_market(MAX_MARKETS_PER_CREATOR - 1));
    }

    #[test]
    fn creator_wallet_limit_rejected() {
        // Once the wallet hits the cap it is rejected.
        assert!(!can_create_market(MAX_MARKETS_PER_CREATOR));
        // Beyond the cap is also rejected.
        assert!(!can_create_market(MAX_MARKETS_PER_CREATOR + 1));
    }

    // ---- Protocol constant smoke tests ----

    #[test]
    fn constants_are_sane() {
        assert_eq!(MIN_STAKE_LAMPORTS, 1_000_000, "0.001 SOL minimum stake");
        assert_eq!(MAX_MARKETS_PER_CREATOR, 3, "max 3 markets per wallet");
        assert_eq!(TRADING_FEE_BPS, 8, "0.08% insurance fee");
    }

    // ---- Account space calculations ----

    #[test]
    fn market_config_space_matches_fields() {
        // Manually computed: 8 discrim + 32 + 32 + 8 + 1 + 1 + 8 + 8 + 2 + 1 + 1 = 102
        assert_eq!(MarketConfig::SPACE, 102);
    }

    #[test]
    fn creator_record_space_matches_fields() {
        // 8 discrim + 32 + 1 + 1 = 42
        assert_eq!(CreatorRecord::SPACE, 42);
    }

    #[test]
    fn registry_space_at_capacity() {
        // 8 + 1 + 4 + 4 + 32*1024 = 17 + 32768 = 32785
        let space = MarketRegistry::space(MAX_REGISTRY_MARKETS);
        assert_eq!(space, 32785);
    }
}
