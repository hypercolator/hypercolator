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
}

#[event]
pub struct MarketCreated {
    pub token_mint: Pubkey,
    pub creator: Pubkey,
    pub tier: u8,
    pub max_leverage_x: u8,
    pub stake_amount: u64,
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use state::{MarketTier, MAX_MARKETS_PER_CREATOR, MIN_STAKE_LAMPORTS, TRADING_FEE_BPS};

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
}
