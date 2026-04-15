use anchor_lang::prelude::Pubkey;

use hypercolator::state::{
    CreatorRecord, MarketConfig, MarketRegistry, MarketTier, MAX_MARKETS_PER_CREATOR,
    MAX_REGISTRY_MARKETS, MIN_STAKE_LAMPORTS, TRADING_FEE_BPS,
};

// --- Tier assignment ---

#[test]
fn test_tier_assignment_major_tokens() {
    let usdc: Pubkey = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
        .parse()
        .unwrap();
    assert_eq!(MarketTier::from_mint(&usdc), MarketTier::A);
    assert_eq!(MarketTier::A.max_leverage_x(), 20);

    let wsol: Pubkey = "So11111111111111111111111111111111111111112"
        .parse()
        .unwrap();
    assert_eq!(MarketTier::from_mint(&wsol), MarketTier::A);
}

#[test]
fn test_tier_assignment_established_tokens() {
    let msol: Pubkey = "mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So"
        .parse()
        .unwrap();
    assert_eq!(MarketTier::from_mint(&msol), MarketTier::B);
    assert_eq!(MarketTier::B.max_leverage_x(), 10);

    let ray: Pubkey = "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R"
        .parse()
        .unwrap();
    assert_eq!(MarketTier::from_mint(&ray), MarketTier::B);
}

#[test]
fn test_tier_assignment_unknown_tokens() {
    let pump = Pubkey::new_unique();
    assert_eq!(MarketTier::from_mint(&pump), MarketTier::C);
    assert_eq!(MarketTier::C.max_leverage_x(), 5);
}

// --- Protocol constants ---

#[test]
fn test_protocol_constants() {
    assert_eq!(MIN_STAKE_LAMPORTS, 1_000_000);
    assert_eq!(MAX_MARKETS_PER_CREATOR, 3);
    assert_eq!(TRADING_FEE_BPS, 8);
    assert_eq!(MAX_REGISTRY_MARKETS, 256);
}

// --- Account space invariants ---

#[test]
fn test_account_space_market_config() {
    // 8 discrim + 32 + 32 + 8 + 1 + 1 + 8 + 8 + 2 + 1 + 1 = 102
    assert_eq!(MarketConfig::SPACE, 102);
}

#[test]
fn test_account_space_creator_record() {
    // 8 discrim + 32 + 1 + 1 = 42
    assert_eq!(CreatorRecord::SPACE, 42);
}

#[test]
fn test_account_space_registry_at_cap() {
    // 8 + 1 + 4 + 4 + 32*256 = 8209 (< 10240 CPI data-increase limit)
    assert_eq!(MarketRegistry::space(MAX_REGISTRY_MARKETS), 8_209);
}

// --- PDA determinism ---

#[test]
fn test_market_pda_deterministic() {
    let mint = Pubkey::new_unique();
    let (a, bump_a) =
        Pubkey::find_program_address(&[b"market", mint.as_ref()], &hypercolator::id());
    let (b, bump_b) =
        Pubkey::find_program_address(&[b"market", mint.as_ref()], &hypercolator::id());
    assert_eq!(a, b);
    assert_eq!(bump_a, bump_b);
}

#[test]
fn test_registry_pda_deterministic() {
    let (a, bump_a) =
        Pubkey::find_program_address(&[b"registry"], &hypercolator::id());
    let (b, bump_b) =
        Pubkey::find_program_address(&[b"registry"], &hypercolator::id());
    assert_eq!(a, b);
    assert_eq!(bump_a, bump_b);
}

#[test]
fn test_creator_record_pda_per_wallet() {
    let w1 = Pubkey::new_unique();
    let w2 = Pubkey::new_unique();
    let (p1, _) =
        Pubkey::find_program_address(&[b"creator", w1.as_ref()], &hypercolator::id());
    let (p2, _) =
        Pubkey::find_program_address(&[b"creator", w2.as_ref()], &hypercolator::id());
    assert_ne!(p1, p2);
}
