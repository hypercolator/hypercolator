# HYPERCOLATOR - MASTER PLAN

> Decentralized Perpetual Futures Exchange on Solana
> Based on Percolator by Anatoly Yakovenko (Toly)

---

## PERMANENT RULES (apply to every file, every task, forever)

1. **No em dash or double dash in prose/text.** Never write `--` or the em dash character as punctuation in any project file, doc, comment, or string. Use single dash `-` only. Exception: CLI flags like `cargo --lib`, `pnpm --filter` are fine as they are command syntax, not punctuation.

2. **All project output must be in English.** Code, comments, docs, commit messages, PR descriptions, UI text - all English. Regardless of what language the user prompts in.

3. **Never add back button or home button to any UI.** Navigation is handled by the app shell or browser. Do not add explicit "Back" or "Home" buttons to any page or component.

4. **404 pages must be user-friendly, not developer-facing.** No stack traces, no error codes, no technical jargon. Show a clear human message and a helpful action. Style it consistently with the rest of the app.

---

## PROJECT SUMMARY

Hypercolator is a fork of Percolator developed into:
- A permissionless perpetual futures DEX (anyone can create a market)
- Supports pump.fun tokens without external oracle
- On-chain AMM-based TWAP price system
- Self-funded insurance model - team provides zero capital
- Next.js frontend with Solana Wallet Adapter
- Automated keeper bot for liquidations and funding

---

## FINANCIAL MODEL - SELF-FUNDED INSURANCE

### Core Concept
The team provides zero insurance capital. The reserve grows automatically from a fee on every trade.

### Fee Flow
```
Every trade executes
        |
        v
0.1% fee deducted automatically from position
        |
   +----+----+
   v         v
 0.08%     0.02%
Insurance  Protocol
  Fund      Fee
(on-chain) (to team)
```

### Fee Structure

| Type | Amount | Destination | Purpose |
| --- | --- | --- | --- |
| Insurance fee | 0.08% per trade | Insurance Fund on-chain | Bad debt reserve |
| Protocol fee | 0.02% per trade | Team wallet | Operational revenue |
| Market creation stake | ~$10 USDC | Per-market Insurance Fund | Seed per market |

### Bad Debt Resolution Order

```
1. Keeper bot liquidates position before collateral runs out  (most common)
        | (if too late)
        v
2. ADL - most profitable opposing position force-closed partially
        | (if still not enough)
        v
3. Insurance Fund (accumulated from fees) covers the remainder
        | (if fund empty - extremely rare)
        v
4. Socialized loss - small haircut to all winners in that market
```

### Insurance Fund Growth Projection

| Daily Volume | Fee per Day | Fund after 1 Month |
| --- | --- | --- |
| $10,000 | $8 | ~$240 |
| $100,000 | $80 | ~$2,400 |
| $1,000,000 | $800 | ~$24,000 |

The busier the exchange, the thicker the fund - automatically.

### Team Capital Required

| Item | Estimate | Notes |
| --- | --- | --- |
| Solana program deploy | 2-5 SOL | One-time |
| Keeper bot server | ~$10-20/month | Cheap VPS |
| **Total** | **< $1,000** | No insurance capital from team |

---

## PREREQUISITES - MANUAL SETUP REQUIRED

### GitHub App (App ID + PEM)

1. Go to: https://github.com/settings/apps/new
2. App name: `hypercolator-bot`
3. Homepage URL: any URL
4. Disable Webhook
5. Set Permissions:
   - Contents - Read and Write
   - Issues - Read and Write
   - Pull Requests - Read and Write
   - Metadata - Read (required)
6. Click "Create GitHub App"
7. Note the **App ID** (number on the App page)
8. Scroll down - "Generate a private key" - download the `.pem` file
9. Install App to your GitHub account:
   - App settings - Install App - select your account
   - Note the **Installation ID** from the URL (`https://github.com/settings/installations/XXXXXXXXX`)

### Replit Secrets (required before build)

| Secret Name | Value |
| --- | --- |
| `GITHUB_APP_ID` | App ID number from step 7 |
| `GITHUB_APP_PRIVATE_KEY` | Full contents of the .pem file (paste directly) |
| `GITHUB_APP_INSTALLATION_ID` | Installation ID number from step 9 |
| `GITHUB_USERNAME` | Your GitHub username |

---

## TASK DEPENDENCY MAP

```
Task #8:  GitHub App Setup and Repository Bootstrap
    |
    v
Task #9:  Percolator Research and Architecture
    |
    v
Task #10: Rust/Solana Toolchain and Anchor Scaffold
    |
    v
Task #11: Market Factory + Fee and Insurance System
   |----------------+----------------+
   v                v                v
Task #12:       Task #13:       Task #14:
Price Engine    Lifecycle and   Frontend
TWAP + AMM      Tier System     MVP (Next.js)
   |                |
   +--------+-------+
            v
        Task #15:
     Keeper Bot and
     GitHub Automation
     (open issue, PR)
```

---

## TASK DETAILS

---

### TASK #8 - GitHub App Setup and Repository Bootstrap
**Depends on:** nothing (start here)

**Work:**
- Build TypeScript helper module for GitHub App authentication
  (JWT generation from App ID + PEM, exchange for Installation Token)
- Fork `aeyakovenko/percolator` to user GitHub account via GitHub API
- Clone fork to workspace: `hypercolator/`
- Create branch `hypercolator-feature` on the fork
- Open Issue #1 on Toly's repo: "Architecture feedback request - Hypercolator extension"
- Set up `scripts/github/` package for all GitHub operations

**Secrets used:**
- GITHUB_APP_ID
- GITHUB_APP_PRIVATE_KEY
- GITHUB_APP_INSTALLATION_ID
- GITHUB_USERNAME

**Done when:**
- Fork exists at `github.com/<username>/percolator`
- Branch `hypercolator-feature` exists on the fork
- Issue is open on Toly's repo
- `scripts/github/src/auth.ts` generates a token without error

---

### TASK #9 - Percolator Research and Architecture
**Depends on:** Task #8

**Work:**
- Read spec.md (~112KB) from Toly's repo
- Read full source: percolator.rs, i128.rs, wide_math.rs
- Catalog all public types, enums, function signatures
- Write `docs/percolator-architecture.md` containing:
  - Text-based architecture diagram
  - All modules and their roles
  - How liquidation works (ADL system)
  - How funding rate is calculated
  - How PnL warmup works
  - Limitations: this is a pure Rust library, NOT a Solana program
  - Adaptation plan: what Hypercolator must build from scratch

**Done when:**
- `docs/percolator-architecture.md` exists and is complete
- Clear section on "what Hypercolator must build from scratch"

---

### TASK #10 - Rust/Solana Toolchain and Anchor Scaffold
**Depends on:** Task #9

**Work:**
- Install Rust stable + Solana CLI + Anchor framework via Nix/replit.nix
- Initialize Anchor project: `anchor init hypercolator` inside `hypercolator/`
- Vendor Percolator library as local Cargo crate: `hypercolator/crates/percolator/`
- Configure workspace Cargo.toml to include percolator as a dependency
- Verify: `anchor build`, `cargo test --lib`, `cargo fmt` all succeed
- Push initial scaffold to branch `hypercolator-feature` on the fork

**Done when:**
- `anchor build` succeeds without error
- `cargo test --lib` passes
- `hypercolator/` folder exists in workspace and is pushed to GitHub

---

### TASK #11 - Market Factory + Fee and Insurance System
**Depends on:** Task #10

**Market Factory:**
- Instruction `create_market(token_mint, stake_amount)`
- Account structs: MarketConfig (PDA), MarketRegistry, CreatorRecord
- Validation: minimum stake ~$10, max 3 markets per wallet, valid SPL mint
- Tier assignment: Tier A (50x), Tier B (20x), Tier C pump.fun (5x)

**Fee and Insurance System (team provides zero capital):**
- `InsuranceFund` account on-chain per market - holds accumulated fees
- Every trade instruction:
  - Deducts 0.08% - goes to `InsuranceFund` account
  - Deducts 0.02% - goes to `ProtocolFeeVault` (team wallet)
- Market creation stake (~$10) - goes to that market's `InsuranceFund` as seed
- `pay_from_insurance(amount)` - internal only, called on bad debt
- Insurance fund balance is public and readable on-chain

**Done when:**
- `create_market` succeeds and market is registered
- Every trade auto-deducts fee and distributes correctly
- Insurance fund balance is readable on-chain
- 5+ unit tests pass

---

### TASK #12 - Price Engine: TWAP and Anti-Manipulation
**Depends on:** Task #11

**Work:**
- Module `price_engine.rs`:
  - Read spot price from AMM pool (Raydium/Orca compatible, x*y=k)
  - Accumulate TWAP over 30-minute window
  - Anti-manipulation: min liquidity threshold, max deviation 20%
- Instruction `update_twap` for keeper to call each slot
- Wire TWAP into `liquidate` instruction
- Unit tests: TWAP math, deviation rejection, min liquidity

**Done when:**
- TWAP updated via `update_twap` instruction
- Liquidation uses validated TWAP price
- All unit tests pass

---

### TASK #13 - Market Lifecycle and Tier System
**Depends on:** Task #11

**Work:**
- `status` field on MarketConfig: Inactive, Active, Expired
- `activate_market`: Inactive to Active on first trade
- `expire_market`: Active to Expired after 48h inactivity
- `last_activity_ts` updated on every trade
- Per-trade leverage cap enforcement by tier
- Unit tests: all state transitions, leverage cap enforcement

**Done when:**
- Market lifecycle follows the state machine
- Trading on Expired market is rejected
- Leverage cap enforced by tier

---

### TASK #14 - Frontend MVP: Next.js + Wallet Adapter
**Depends on:** Task #11

**Stack:**
- Next.js 14 App Router
- @solana/wallet-adapter (Phantom, Backpack, Solflare)
- @coral-xyz/anchor for on-chain calls
- Tailwind CSS

**Pages:**
- `/` - All markets list (price, volume, status, tier badge)
- `/markets/[address]` - Market detail + long/short trading panel
- `/create` - Create new market form (paste token address)

**Features:**
- Connect/disconnect wallet
- Create market: paste token, preview tier/stake, submit on-chain
- Trade: long/short, size, leverage slider (capped by tier), submit
- Insurance fund balance displayed per market (transparent)
- Orderbook display
- Dark theme, professional DEX feel
- User-friendly 404 page (no stack traces, no error codes)
- No back button, no home button on any page

**Done when:**
- All 3 pages accessible
- Wallet connect works
- Create market sends on-chain transaction
- Trading panel sends long/short transaction

---

### TASK #15 - Keeper Bot and GitHub Automation
**Depends on:** Task #12, Task #13

**Keeper Bot (scripts/keeper/):**
- Node.js polling loop every ~400ms (1 Solana slot)
- Crank `update_twap` for all active markets
- Check under-collateralized positions, call `liquidate`
  (insurance fund pays first, then ADL)
- Expire markets inactive for 48h

**GitHub Automation:**
- Uses GitHub App auth (App ID + PEM)
- Auto-generate PR to Toly's repo with documentation improvements
- Open issue for architecture feedback
- CI: `.github/workflows/ci.yml` (`cargo test`, `cargo fmt`, `cargo clippy`)

**Done when:**
- Keeper runs and can be started via workflow
- PR drafted on fork, ready to submit upstream
- CI pipeline green

---

## IMPORTANT NOTES

1. **Team provides zero insurance capital.** Reserve grows automatically from 0.08% fee on every trade.

2. **Percolator is a pure Rust library** - not a Solana program. Hypercolator takes its risk engine logic and wraps it in Anchor instructions that run on-chain.

3. **Pump.fun tokens have no oracle.** Solution: TWAP from AMM pool directly on-chain.

4. **GitHub App cannot be banned** like a PAT because its identity is separate from a personal account.

5. **Do not push breaking changes upstream.** Contributions to Toly's repo are documentation, examples, and modular extensions only.

6. **Insurance fund is transparent.** Anyone can see its balance on-chain, building user trust without the team needing to prove anything.
