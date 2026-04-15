# Percolator Architecture — Research Reference

**Source:** aeyakovenko/percolator @ master  
**Spec version:** v12.17.0  
**Purpose:** Reference document for Hypercolator's Anchor implementation

---

## 0. What Percolator Is

Percolator is a pure Rust `no_std` library implementing a formally verified risk engine for a single-vault perpetual DEX. It is:

- A **library crate** only - no binary, no CLI, no deployment tooling
- Target-agnostic: designed to compile to both native x86 and Solana BPF/SBF
- Verified with **Kani** (formal model checker) for safety and liveness properties
- Scope: one quote-token vault, one market, up to 4096 accounts in production

It is **not** a Solana program. There are no `#[program]` attributes, no Anchor macros, no IDL, no PDA derivation, no token program calls, and no CLI or migration scripts. Percolator is purely the economic math layer.

---

## 1. Module Structure

```
src/
  lib.rs          - re-exports (14 lines, trivial)
  percolator.rs   - main risk engine (4881 lines)
  i128.rs         - BPF-safe 128-bit integer types I128/U128 (927 lines)
  wide_math.rs    - 256-bit transient arithmetic helpers (2067 lines)

tests/
  unit_tests.rs         - behavioral unit tests (3967 lines)
  fuzzing.rs            - property-based fuzzing
  amm_tests.rs          - AMM-specific tests
  proofs_*.rs           - Kani formal proof harnesses (8 files)
```

The three source modules form a strict dependency chain:
- `wide_math` depends on `i128`
- `percolator` depends on both `i128` and `wide_math`

---

## 2. All Public Types and Structs

### 2.1 Primary State: `RiskEngine`

The single struct holding all market state. Designed for zero-copy (`#[repr(C)]`) Solana account storage.

```
RiskEngine {
    vault: U128,                        // total vault tokens V
    insurance_fund: InsuranceFund,      // insurance balance I
    params: RiskParams,                 // immutable configuration
    current_slot: u64,                  // latest slot seen

    market_mode: MarketMode,            // Live or Resolved
    resolved_price: u64,                // settlement price
    resolved_slot: u64,
    resolved_payout_h_num: u128,        // locked haircut numerator for resolved payouts
    resolved_payout_h_den: u128,
    resolved_payout_ready: u8,          // 0 = not ready, 1 = snapshot locked
    resolved_k_long_terminal_delta: i128,
    resolved_k_short_terminal_delta: i128,
    resolved_live_price: u64,

    c_tot: U128,                        // sum of all capital C_i
    pnl_pos_tot: u128,                  // sum of all max(PNL_i, 0)
    pnl_matured_pos_tot: u128,          // subset of above that has warmed up

    // ADL side indices
    adl_mult_long: u128,    // A_long - position multiplier (starts at ADL_ONE)
    adl_mult_short: u128,   // A_short
    adl_coeff_long: i128,   // K_long - cumulative P/L coefficient
    adl_coeff_short: i128,  // K_short
    adl_epoch_long: u64,    // epoch counter for full-drain resets
    adl_epoch_short: u64,
    adl_epoch_start_k_long: i128,    // K snapshot at epoch start (for stale accounts)
    adl_epoch_start_k_short: i128,
    f_long_num: i128,       // F_long_num - cumulative funding numerator
    f_short_num: i128,      // F_short_num
    f_epoch_start_long_num: i128,
    f_epoch_start_short_num: i128,

    oi_eff_long_q: u128,    // effective long open interest
    oi_eff_short_q: u128,   // effective short open interest (must equal long)
    side_mode_long: SideMode,
    side_mode_short: SideMode,
    stored_pos_count_long: u64,
    stored_pos_count_short: u64,
    stale_account_count_long: u64,  // accounts with epoch_snap one behind
    stale_account_count_short: u64,

    phantom_dust_bound_long_q: u128,   // upper bound on A-decay rounding dust
    phantom_dust_bound_short_q: u128,

    materialized_account_count: u64,
    neg_pnl_account_count: u64,     // O(1) readiness check for resolved close
    last_oracle_price: u64,         // P_last
    fund_px_last: u64,
    last_market_slot: u64,
    last_crank_slot: u64,
    gc_cursor: u16,

    // Slab (flat array of accounts with bitmap + freelist)
    used: [u64; BITMAP_WORDS],
    num_used_accounts: u16,
    free_head: u16,
    next_free: [u16; MAX_ACCOUNTS],
    accounts: [Account; MAX_ACCOUNTS],
}
```

### 2.2 Per-Account State: `Account`

```
Account {
    capital: U128,              // C_i: protected principal
    kind: u8,                   // 0 = User, 1 = LP
    pnl: i128,                  // PNL_i: realized PnL
    reserved_pnl: u128,         // R_i: warmed-up reserved PnL
    position_basis_q: i128,     // basis_pos_q_i: signed fixed-point position (base * POS_SCALE)
    adl_a_basis: u128,          // a_basis_i: A_side snapshot at attachment
    adl_k_snap: i128,           // k_snap_i: K_side snapshot at attachment
    f_snap: i128,               // f_snap_i: F_side_num snapshot at attachment
    adl_epoch_snap: u64,        // epoch at attachment

    matcher_program: [u8; 32],  // LP matching engine program (for LP accounts)
    matcher_context: [u8; 32],
    owner: [u8; 32],            // owner pubkey

    fee_credits: I128,          // <= 0; negative = outstanding fee debt

    // Two-bucket warmup reserve
    sched_present: u8,          // 1 = scheduled bucket exists
    sched_remaining_q: u128,    // unreleased amount in scheduled bucket
    sched_anchor_q: u128,       // original amount (for linear release math)
    sched_start_slot: u64,      // when scheduled bucket was created
    sched_horizon: u64,         // full maturity duration in slots
    sched_release_q: u128,      // already-released amount (progress cursor)

    pending_present: u8,        // 1 = pending bucket exists
    pending_remaining_q: u128,  // pending amount (not yet promoted)
    pending_horizon: u64,       // max horizon of all merged pending deposits
    pending_created_slot: u64,
}
```

### 2.3 Configuration: `RiskParams`

All fields are **immutable for the lifetime of the market** (spec requirement 28).

```
RiskParams {
    maintenance_margin_bps: u64,    // margin for liquidation eligibility
    initial_margin_bps: u64,        // margin for new risk-increasing trades
    trading_fee_bps: u64,           // fee charged per trade (both sides)
    max_accounts: u64,              // active deployment account limit
    new_account_fee: U128,          // cost to open a new account
    max_crank_staleness_slots: u64, // how stale a crank can be
    liquidation_fee_bps: u64,       // liquidation penalty rate
    liquidation_fee_cap: U128,      // max liquidation fee per event
    min_liquidation_abs: U128,      // min liquidation fee per event
    min_initial_deposit: U128,      // minimum first deposit to materialize account
    min_nonzero_mm_req: u128,       // absolute floor for maintenance margin
    min_nonzero_im_req: u128,       // absolute floor for initial margin
    insurance_floor: U128,          // insurance balance cannot be spent below this
    h_min: u64,                     // minimum warmup horizon in slots
    h_max: u64,                     // maximum warmup horizon in slots
    resolve_price_deviation_bps: u64, // max deviation for resolved_price vs live
}
```

### 2.4 Insurance Fund: `InsuranceFund`

```
InsuranceFund { balance: U128 }
```

Simple balance. The floor is stored in `RiskParams.insurance_floor`.

### 2.5 Instruction Context: `InstructionContext`

Per-instruction ephemeral state used for deferred reset scheduling and touched-account tracking.

```
InstructionContext {
    pending_reset_long: bool,
    pending_reset_short: bool,
    h_lock_shared: u64,                          // warmup horizon for this instruction
    touched_accounts: [u16; 64],                 // MAX_TOUCHED_PER_INSTRUCTION = 64
    touched_count: u8,
}
```

### 2.6 Enums

```rust
// Market lifecycle
enum MarketMode { Live = 0, Resolved = 1 }

// ADL side lifecycle
enum SideMode { Normal = 0, DrainOnly = 1, ResetPending = 2 }

// How to handle new positive PnL when writing PNL_i
enum ReserveMode {
    UseHLock(u64),          // put in warmup queue with this horizon
    ImmediateRelease,       // immediately counted as matured
    NoPositiveIncreaseAllowed,
}

// Position direction
enum Side { Long, Short }

// Liquidation scope
enum LiquidationPolicy { FullClose, ExactPartial(u128) }

// Result of force_close_resolved
enum ResolvedCloseResult {
    ProgressOnly,           // local reconciled, payout not yet available
    Closed(u128),           // account closed, payout in quote tokens
}

// Errors
enum RiskError {
    InsufficientBalance,
    Undercollateralized,
    Unauthorized,
    PnlNotWarmedUp,
    Overflow,
    AccountNotFound,
    SideBlocked,
    CorruptState,
}
```

---

## 3. Key Constants

| Constant | Value | Meaning |
|---|---|---|
| `POS_SCALE` | 1_000_000 | Fixed-point scale for positions (1 base = 1e6 units) |
| `ADL_ONE` | 1_000_000_000_000_000 (10^15) | Scale for A_side index |
| `MIN_A_SIDE` | 100_000_000_000_000 (10^14) | A_side below this triggers DrainOnly |
| `FUNDING_DEN` | 1_000_000_000 (10^9) | Scale for funding rate (parts per billion per slot) |
| `MAX_ACCOUNTS` | 4096 (prod) / 64 (test) / 4 (kani) | Slab capacity |
| `MAX_TOUCHED_PER_INSTRUCTION` | 64 | Accounts finalized per instruction |
| `GC_CLOSE_BUDGET` | 32 | Max GC closes per crank |
| `ACCOUNTS_PER_CRANK` | 128 | Max accounts processed per keeper crank |
| `MAX_VAULT_TVL` | 10^16 | Max total vault balance |
| `MAX_ORACLE_PRICE` | 10^12 | Max acceptable price value |
| `MAX_POSITION_ABS_Q` | 10^14 | Max absolute position in scaled units |
| `MAX_OI_SIDE_Q` | 10^14 | Max open interest per side |
| `MAX_MATERIALIZED_ACCOUNTS` | 1_000_000 | Global materialization cap |
| `MAX_ABS_FUNDING_E9_PER_SLOT` | 10^9 | Max |funding_rate| per slot |
| `MAX_WARMUP_SLOTS` | u64::MAX | Max warmup horizon |

---

## 4. How the A/K/F Lazy Settlement Works

Percolator uses lazy global indices instead of iterating all accounts on every event. This is the mathematical core that enables O(1) settlement.

### 4.1 Concept

Every position-affecting event on one side has the form:
- Position scaled by factor `alpha`: `q_new = alpha * q_old`
- PnL delta of `beta` per unit position: `pnl_delta = beta * q / POS_SCALE`

Rather than apply this to every account immediately, the engine accumulates:
- `A_side = product of all alpha factors` (starts at ADL_ONE = 10^15)
- `K_side = sum of (A_old * beta)` (cumulative PnL coefficient)
- `F_side_num = cumulative funding contribution`

Each account stores snapshots `(a_basis_i, k_snap_i, f_snap_i, epoch_snap_i)` taken at the last time the account was explicitly settled. On-demand settlement computes:

```
effective_abs_pos_q = floor(abs(basis_pos_q_i) * A_side / a_basis_i)

pnl_delta = floor(
    abs(basis_pos_q_i) *
    (((K_side - k_snap_i) * FUNDING_DEN) + (F_side_num - f_snap_i))
    / (a_basis_i * POS_SCALE * FUNDING_DEN)
)
```

This formula combines mark-to-market (via K_side) and funding (via F_side_num) into one exact 256-bit computation.

### 4.2 Epoch System

When `A_side` decays too far below `MIN_A_SIDE`, the engine initiates a full side reset:
1. `DrainOnly` mode: no new OI-increasing trades allowed
2. When `OI_eff_side == 0`: `begin_full_drain_reset()` - increments epoch, resets A_side to ADL_ONE
3. Stale accounts (those with `epoch_snap_i = epoch_s - 1`) must be individually settled
4. Once `stale_account_count_s == 0`: `finalize_side_reset()` - returns to Normal mode

**Epoch-gap invariant**: an account's epoch snapshot can be at most 1 behind the current epoch. Gaps of 2 or more are treated as corruption.

---

## 5. How PnL Warmup Works

Fresh profits are locked in a "reserve" that matures over time to prevent instant withdrawal of manipulated PnL.

### 5.1 Two-Bucket Design

Each account has two reserve buckets:
- **Scheduled bucket**: the older active bucket; matures linearly over `sched_horizon` slots
- **Pending bucket**: accumulates new reserves; cannot mature until promoted

When scheduled bucket completes, pending is promoted to scheduled.

### 5.2 Linear Maturity Formula

```
matured_fraction = min(elapsed, sched_horizon) / sched_horizon
released_so_far = floor(sched_anchor_q * elapsed / sched_horizon)
new_release = released_so_far - sched_release_q
```

### 5.3 Reserve Modes

When `set_pnl` is called with a positive PnL increase:
- `UseHLock(h)` - new PnL enters the warmup queue (locked for h slots)
- `ImmediateRelease` - PnL counts as immediately matured (used for resolved markets)
- `NoPositiveIncreaseAllowed` - fail if PnL would increase (used during loss settlement)

### 5.4 Key Invariants

- `R_i <= max(PNL_i, 0)` always
- `PNL_matured_pos_tot <= PNL_pos_tot` always
- Fresh reserve NEVER inherits elapsed time from an older scheduled bucket
- Reserve loss is consumed newest-first (pending before scheduled)

---

## 6. How Haircuts Work

When the system is undercollateralized, positive PnL claims are haircut proportionally.

### 6.1 Matured Withdrawal Haircut `h`

Controls how much of released PnL can be withdrawn or auto-converted to capital:
```
h_num = min(Residual, PNL_matured_pos_tot)
h_den = PNL_matured_pos_tot
Residual = max(0, V - C_tot - I)
```
When `h = 1` (fully backed), auto-conversion is allowed. Otherwise users must explicitly call `convert_released_pnl`.

### 6.2 Trade Collateral Haircut `g`

Controls how much positive PnL can back a new risk-increasing trade:
```
g_num = min(Residual, PNL_pos_tot)
g_den = PNL_pos_tot
PNL_eff_trade_i = floor(PosPNL_i * g_num / g_den)
```

The trade approval check uses a counterfactual version that strips the candidate trade's own positive slippage from both the account's PnL and the global aggregate - preventing a position from bootstrapping itself.

---

## 7. How Liquidation and ADL Work

### 7.1 Liquidation Eligibility

After touching an account:
- `Eq_net_i = max(0, C_i + PNL_i - FeeDebt_i)` (maintenance equity)
- `MM_req_i = max(floor(Notional_i * maintenance_bps / 10000), min_nonzero_mm_req)`
- Liquidatable when `Eq_net_i <= MM_req_i`

### 7.2 Liquidation Execution

1. Touch account (settle PnL delta, warm up reserves, settle losses)
2. Verify eligible
3. Close position synthetically at `oracle_price` - NO execution slippage (price = oracle)
4. Settle losses from capital
5. Charge liquidation fee via `charge_fee_to_insurance`
6. Compute bankruptcy deficit `D = max(-PNL_i, 0)`
7. Call `enqueue_adl(ctx, side, q_close, D)`

### 7.3 ADL (Auto-Deleveraging) via `enqueue_adl`

When a bankrupt liquidation leaves deficit D:
1. Spend insurance down to `I_floor` first
2. If opposing OI exists, compute `delta_K_abs = ceil(D_rem * A_opp * POS_SCALE / OI_opp)` using exact 256-bit arithmetic
3. Apply `K_opp -= delta_K_abs` (socializes loss to all opposing positions proportionally)
4. Scale down `A_opp = floor(A_opp * OI_post / OI_before)`
5. If `A_opp < MIN_A_SIDE`, set `mode_opp = DrainOnly`
6. If K-space would overflow `i128`, route remainder to `record_uninsured_protocol_loss` (global haircut h)

This is lazy: individual opposing accounts are only settled when explicitly touched - they don't need to be iterated immediately.

---

## 8. How Funding Rate Works

### 8.1 Funding Accrual

Called in every live operation via `accrue_market_to(now_slot, oracle_price, funding_rate_e9_per_slot)`:

1. Mark-to-market: `K_side += A_side * (oracle_price - P_last)` for each side with OI
2. Funding: if both sides have OI and rate != 0 and dt > 0:
   - `fund_num_total = fund_px_last * funding_rate_e9_per_slot * dt` (exact 256-bit)
   - `F_long_num -= A_long * fund_num_total`
   - `F_short_num += A_short * fund_num_total`

### 8.2 Who Supplies the Funding Rate?

**The wrapper** - Percolator does not compute funding rates. The `funding_rate_e9_per_slot` input is a wrapper-owned policy input (parts per billion per slot). The wrapper must:
- Implement a mark/index price oracle comparison
- Derive the rate (e.g. premium / clamp / EWMA model)
- Pass it as a trusted input to each instruction

This is Hypercolator's primary extension point.

---

## 9. Public API (All External Instructions)

All are methods on `&mut RiskEngine`. Methods suffixed `_not_atomic` can return `Err` after partial mutation and MUST abort the transaction. Methods without the suffix use validate-then-mutate.

### 9.1 Capital Flow (pure, no `accrue_market_to`)

| Method | Description |
|---|---|
| `deposit(idx, amount, now_slot)` | Open or add to account. Materializes if `amount >= min_initial_deposit` |
| `deposit_fee_credits(idx, amount, now_slot)` | Repay outstanding fee debt |
| `top_up_insurance_fund(amount, now_slot)` | Add to insurance fund |
| `charge_account_fee(idx, fee_abs, now_slot)` | Optional wrapper-side fee |
| `settle_flat_negative_pnl(idx, now_slot)` | Clean up flat account with negative PnL |

### 9.2 Live Market Operations (call `accrue_market_to` internally)

| Method | Description |
|---|---|
| `settle_account(idx, oracle, slot, rate, h_lock)` | Force-settle account state |
| `withdraw(idx, amount, oracle, slot, rate, h_lock)` | Withdraw capital (IM check) |
| `convert_released_pnl(idx, x_req, oracle, slot, rate, h_lock)` | Convert warmed PnL to capital |
| `execute_trade(a, b, oracle, slot, rate, h_lock, size_q, exec_price)` | Bilateral trade |
| `liquidate(idx, oracle, slot, rate, h_lock, policy)` | Liquidate eligible account |
| `keeper_crank(slot, oracle, rate, h_lock, candidates[], max)` | Batch settle/liquidate |

### 9.3 Market Lifecycle

| Method | Description |
|---|---|
| `resolve_market(resolved_price, live_oracle, slot, rate)` | **Privileged.** Transition to Resolved mode |
| `force_close_resolved(idx, slot)` | Post-resolve account close (may return ProgressOnly) |
| `reclaim_empty_account(idx, slot)` | Permissionless dust account reclamation |

### 9.4 Standard Live Instruction Lifecycle

Every live state-mutating operation follows this exact order:
1. Validate slot, oracle, funding rate, h_lock bounds
2. Initialize fresh `InstructionContext`
3. Call `accrue_market_to(slot, oracle, rate)` exactly once
4. Set `current_slot = slot`
5. Execute instruction-specific logic
6. Call `finalize_touched_accounts_post_live(ctx)` - auto-converts flat accounts at h=1, sweeps fee debt
7. Call `schedule_end_of_instruction_resets(ctx)` - dust clearance
8. Call `finalize_end_of_instruction_resets(ctx)` - trigger resets if needed
9. Assert `OI_eff_long == OI_eff_short`

---

## 10. What Percolator Does NOT Have

This section is critical for scoping Hypercolator's build.

| Missing Piece | What Hypercolator Must Build |
|---|---|
| No Solana program | Anchor program wrapping the RiskEngine |
| No oracle | On-chain AMM TWAP (see §12 below) |
| No funding rate computation | Compute from TWAP premium vs mark |
| No market factory | Permissionless `create_market` instruction |
| No token program integration | SPL token vault deposit/withdraw |
| No account authorization | PDA seeds, signer checks, authority |
| No persistent storage layout | Anchor account discriminators, zero-copy |
| No event/log emission | Anchor events for indexing |
| No CLI or scripts | TypeScript SDK, keeper bot |
| No AMM / price discovery | AMM with TWAP for oracle-free pricing |
| No market registry | Registry PDA listing all markets |
| No insurance accrual rule | The 0.08% fee that builds insurance |
| No market creation governance | Who can create, what params are allowed |
| No TWAP manipulation protection | Min observation window, staleness check |

---

## 11. State Machine Diagrams

### 11.1 Market Lifecycle

```
         create_market
              |
              v
           [Live] ----> [DrainOnly (side)] ----> [ResetPending (side)]
              |                                         |
              | resolve_market (privileged)      finalize_side_reset
              |                                         |
              v                                         v
          [Resolved]                               [Normal (side)]
              |
              | force_close_resolved * N
              v
           [Empty]
```

### 11.2 Account Lifecycle

```
  missing
    |
    | deposit >= MIN_INITIAL_DEPOSIT
    v
  materialized (flat, C_i > 0, PNL=0)
    |
    | execute_trade
    v
  open position
    |
    | execute_trade (close) or liquidate
    v
  flat with PnL
    |
    | auto-convert (h=1) or convert_released_pnl
    | + withdraw
    v
  reclaimable (C_i < MIN_INITIAL_DEPOSIT, PNL=0)
    |
    | reclaim_empty_account
    v
  missing (slot reused)
```

### 11.3 PnL Warmup Bucket Lifecycle

```
  new positive PnL
      |
      | UseHLock(h) > 0
      v
  [pending bucket]  <-- new reserves merge here if scheduled exists
      |
      | promote_pending_to_scheduled (when scheduled is empty)
      v
  [scheduled bucket]  -- linear release over h slots
      |
      | advance_profit_warmup
      v
  matured (PNL_matured_pos_tot increases)
      |
      | consume_released_pnl + set_capital (h=1 auto-convert)
      | or convert_released_pnl (explicit)
      v
  capital (C_i)
```

---

## 12. Equity Lanes

Percolator defines three distinct equity concepts, each used for a specific gate:

| Lane | Formula | Used for |
|---|---|---|
| `Eq_maint_raw_i` | `C_i + PNL_i - FeeDebt_i` | Maintenance check (liquidation) |
| `Eq_trade_open_raw_i` | `C_i + min(PNL_i-slippage, 0) + PNL_eff_trade_open_i - FeeDebt_i` | Risk-increasing trade approval |
| `Eq_withdraw_raw_i` | `C_i + min(PNL_i, 0) + PNL_eff_matured_i - FeeDebt_i` | Withdrawal approval |

All are computed in exact 256-bit signed arithmetic. The trade-open lane uses a counterfactual version of PnL that strips the candidate trade's own positive slippage to prevent self-bootstrapping.

---

## 13. Arithmetic Safety Design

Percolator enforces several layers of arithmetic safety:

1. **No native `i128` multiplication for wide products** - all wide products use `wide_math::U256`/`I256`
2. **No rounding without floor semantics** - all signed division uses `floor_div_signed_conservative`
3. **No unsafe code** - `#![forbid(unsafe_code)]`
4. **No std** - `#![no_std]` (no heap allocation, no panic unwinding)
5. **Checked arithmetic everywhere** - overflow returns `RiskError::Overflow`, never wraps
6. **BPF-safe 128-bit types** - `I128`/`U128` use `[u64; 2]` to avoid BPF ABI issues
7. **Conservation invariant** - `V >= C_tot + I` is checked at the end of every instruction

The `i128.rs` module provides `I128` and `U128` as `[u64; 2]` structs to work around Solana BPF's inability to reliably handle native `i128`/`u128` in certain ABI positions.

---

## 14. Adaptation Plan - What Hypercolator Builds from Scratch in Anchor

### 14.1 Anchor Program Layer

Wrap every `RiskEngine` method in an Anchor instruction handler:
- PDA derivation for market state (zero-copy `RiskEngine` account)
- SPL token vault PDA for deposit/withdraw
- Authority checks (market creator, keeper, user)
- Anchor event emission for indexing
- CU budget management (256-bit math is expensive)

### 14.2 Permissionless Market Factory

```
create_market(
    base_mint: Pubkey,
    initial_price: u64,
    params: RiskParams,
) -> market_pda
```

- Market PDA seeded by `base_mint` + `base_mint`
- Any token including pump.fun mints
- Market registry list PDA for UI discovery

### 14.3 On-Chain AMM TWAP Oracle

Since Percolator does not include an oracle, Hypercolator must implement one. The design:

```
AMM state (per market):
    sqrt_price: u128          - current price as sqrt(price) * 2^64
    liquidity: u128           - active liquidity
    cumulative_price: u128    - sum of price * slots (for TWAP)
    last_update_slot: u64     - last update slot
    observation_window: u64   - required TWAP window in slots

TWAP computation:
    twap = (cumulative_price_now - cumulative_price_then)
           / (slot_now - slot_then)
```

This TWAP value serves as `oracle_price` passed to every Percolator instruction.

**Anti-manipulation**: the TWAP requires a minimum observation window before the price is considered valid for trading. Flash-loan price spikes affect at most one slot's weight.

### 14.4 Funding Rate Computation

```rust
fn compute_funding_rate(mark_price: u64, twap_price: u64) -> i128 {
    // premium = (mark - index) / index
    // clamp to [-MAX_FUNDING_RATE, +MAX_FUNDING_RATE]
    // return as parts per billion per slot (e9 scaling)
}
```

The wrapper calls this and passes the result as `funding_rate_e9_per_slot` to every instruction.

### 14.5 Insurance Accrual (0.08% Fee)

Percolator charges `trading_fee_bps` per trade via `charge_fee_to_insurance`. Setting `trading_fee_bps = 8` (0.08%) causes every trade fee to flow directly into the insurance fund `I`. No separate accrual step needed - this is Percolator's built-in `charge_fee_to_insurance` path.

For Hypercolator, set `trading_fee_bps = 8` in `RiskParams`.

### 14.6 Keeper Bot Architecture

The Percolator spec is designed for permissionless off-chain keepers:
- Off-chain: scan accounts, identify liquidation candidates
- On-chain: call `keeper_crank(slot, oracle, rate, h_lock, candidates, max_revalidations)`
- The engine re-validates each candidate (the keeper's list is untrusted)
- Batch size limited by Solana's 1.4M CU budget (256-bit math is expensive)

### 14.7 What the Wrapper Must Supply

Wrapper-owned inputs that Percolator never derives (must be locked down, not caller-controlled):
- `H_lock` - warmup horizon per instruction
- `funding_rate_e9_per_slot` - computed from TWAP premium
- `oracle_price` - read from on-chain TWAP
- `exec_price` admissibility policy - must stay within 2x fee BPS of oracle

### 14.8 Deployment Parameter Recommendations

For the initial Hypercolator market:

| Parameter | Value | Rationale |
|---|---|---|
| `trading_fee_bps` | 8 | 0.08% - funds insurance |
| `initial_margin_bps` | 1000 | 10% IM |
| `maintenance_margin_bps` | 500 | 5% MM |
| `liquidation_fee_bps` | 100 | 1% liquidation incentive |
| `h_min` | 1 | Minimum 1 slot warmup |
| `h_max` | 604800 | ~7 days at 400ms/slot |
| `min_initial_deposit` | 10_000_000 | 10 USDC (6 decimal) |
| `max_accounts` | 4096 | Full production slab |
| `insurance_floor` | 0 | Can increase post-launch |

---

## 15. Summary: Core Invariants

The engine formally guarantees (from spec §0):

1. **`V >= C_tot + I`** - vault always covers protected capital + insurance
2. **PnL warmup** - fresh positive PnL cannot be withdrawn or back new risk immediately
3. **Fee neutrality** - fees never create claims exceeding collectible headroom
4. **ADL exactness** - bankruptcy losses are socialized lazily but exactly
5. **OI symmetry** - `OI_eff_long == OI_eff_short` at end of every instruction
6. **Conservation** - no tokens are created; only vault tokens can become capital or insurance
7. **Liveness** - liquidation, settlement, and reclamation never require a global scan
8. **No hidden MM** - the protocol does not secretly hold residual inventory
