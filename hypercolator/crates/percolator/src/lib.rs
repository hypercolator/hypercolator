//! Percolator Risk Engine — vendored from aeyakovenko/percolator
//!
//! Pinned commit: 719c408fc4fb3f8388f29b41643abda09523a8d0 (2026-04-14)
//! Spec version: v12.17.0
//!
//! This is a no_std library implementing a formally verified risk engine
//! for a single-vault perpetual DEX. See docs/percolator-architecture.md
//! for the full architecture reference.
//!
//! ## Modules
//!
//! - `percolator` - main RiskEngine, Account, RiskParams, all public types
//! - `i128` - BPF-safe I128/U128 wrapper types (repr [u64; 2])
//! - `wide_math` - 256-bit transient arithmetic (U256/I256)

#![no_std]

pub mod i128;
pub mod percolator;
pub mod wide_math;

// Re-export primary types for convenience
pub use percolator::{
    Account, CrankOutcome, InsuranceFund, InstructionContext, LiquidationPolicy, MarketMode,
    ReserveMode, ResolvedCloseResult, RiskEngine, RiskError, RiskParams, Side, SideMode,
    ADL_ONE, FUNDING_DEN, MAX_ACCOUNTS, MAX_ACCOUNT_NOTIONAL, MAX_OI_SIDE_Q,
    MAX_ORACLE_PRICE, MAX_POSITION_ABS_Q, MAX_VAULT_TVL, MIN_A_SIDE, POS_SCALE,
};
