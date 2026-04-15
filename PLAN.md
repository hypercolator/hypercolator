# HYPERCOLATOR — MASTER PLAN

  > Decentralized Perpetual Futures Exchange on Solana
  > Based on Percolator by Anatoly Yakovenko (Toly)

  ---

  ## RINGKASAN PROYEK

  Hypercolator adalah fork dari Percolator yang dikembangkan menjadi:
  - DEX perpetual futures yang permissionless (siapapun bisa buat market)
  - Mendukung token pump.fun tanpa oracle eksternal
  - Sistem harga TWAP berbasis AMM pool on-chain
  - Frontend Next.js dengan Solana Wallet Adapter
  - Keeper bot otomatis untuk likuidasi & funding

  ---

  ## PREREQUISITE YANG HARUS DISIAPKAN MANUAL

  Sebelum task dimulai, kamu perlu siapkan:

  ### GitHub App (App ID + PEM)
  1. Buka: https://github.com/settings/apps/new
  2. Nama App: `hypercolator-bot`
  3. Homepage URL: isi sembarang
  4. Nonaktifkan Webhook
  5. Set Permissions:
     - Contents → Read & Write
     - Issues → Read & Write
     - Pull Requests → Read & Write
     - Metadata → Read (wajib)
  6. Klik "Create GitHub App"
  7. Catat **App ID** (angka di halaman App)
  8. Scroll ke bawah → "Generate a private key" → download file `.pem`
  9. Install App ke akun GitHub kamu:
     - Masuk ke App settings → Install App → pilih akun kamu
     - Catat **Installation ID** dari URL (`https://github.com/settings/installations/XXXXXXXXX`)

  ### Secrets di Replit (wajib diisi sebelum build)
  | Secret Name | Nilai |
  |---|---|
  | `GITHUB_APP_ID` | Angka App ID dari langkah 7 |
  | `GITHUB_APP_PRIVATE_KEY` | Isi seluruh file .pem (paste langsung) |
  | `GITHUB_APP_INSTALLATION_ID` | Angka Installation ID dari langkah 9 |
  | `GITHUB_USERNAME` | Username GitHub kamu |

  ---

  ## ALUR PENGERJAAN (DEPENDENCY MAP)

  ```
  Task #1: GitHub App Setup & Repository Bootstrap
      │
      ▼
  Task #2: Percolator Research & Architecture
      │
      ▼
  Task #3: Rust/Solana Toolchain & Anchor Scaffold
      │
      ▼
  Task #4: Market Factory On-chain Program
     ├────────────────┬────────────────┐
     ▼                ▼                ▼
  Task #5:        Task #6:        Task #7:
  Price Engine    Lifecycle &     Frontend
  TWAP + AMM      Tier System     MVP (Next.js)
     │                │
     └────────┬───────┘
              ▼
          Task #8:
       Keeper Bot &
       GitHub Automation
       (open issue, PR)
  ```

  ---

  ## DETAIL SETIAP TASK

  ---

  ### TASK #1 — GitHub App Setup & Repository Bootstrap
  **Depends on:** nothing (mulai dari sini)

  **Yang dikerjakan:**
  - Buat helper module TypeScript untuk GitHub App authentication
    (JWT generation dari App ID + PEM → exchange ke Installation Token)
  - Fork repo `aeyakovenko/percolator` ke akun GitHub kamu via GitHub API
  - Clone fork ke workspace: `hypercolator/`
  - Buat branch `hypercolator-feature` di fork
  - Open Issue #1 di repo Toly: "Architecture feedback request — Hypercolator extension"
  - Setup `scripts/github/` package untuk semua GitHub operasi

  **Secrets yang dipakai:**
  - GITHUB_APP_ID
  - GITHUB_APP_PRIVATE_KEY
  - GITHUB_APP_INSTALLATION_ID
  - GITHUB_USERNAME

  **Done kalau:**
  - Fork berhasil ada di `github.com/<username>/percolator`
  - Branch `hypercolator-feature` sudah ada di fork
  - Issue sudah terbuka di repo Toly
  - Script `scripts/github/src/auth.ts` bisa generate token tanpa error

  ---

  ### TASK #2 — Percolator Research & Architecture
  **Depends on:** Task #1

  **Yang dikerjakan:**
  - Baca spec.md (~112KB) dari repo Toly
  - Baca seluruh source: percolator.rs, i128.rs, wide_math.rs
  - Identifikasi semua public types, enums, function signatures
  - Tulis `docs/percolator-architecture.md` berisi:
    - Diagram arsitektur (text-based)
    - Semua modul dan fungsinya
    - Cara market dibuat (tidak ada on-chain — ini simulasi)
    - Cara liquidation bekerja (ADL system)
    - Cara funding rate dihitung
    - Batasan: ini library pure Rust, BUKAN Solana program
    - Adaptation plan: apa yang perlu dibangun from scratch untuk Hypercolator

  **Done kalau:**
  - File `docs/percolator-architecture.md` ada dan lengkap
  - Ada seksi jelas tentang "apa yang harus dibangun Hypercolator dari nol"

  ---

  ### TASK #3 — Rust/Solana Toolchain & Anchor Scaffold
  **Depends on:** Task #2

  **Yang dikerjakan:**
  - Install Rust stable + Solana CLI + Anchor framework via Nix/replit.nix
  - Inisialisasi Anchor project: `anchor init hypercolator` di dalam folder `hypercolator/`
  - Vendor Percolator library sebagai local Cargo crate: `hypercolator/crates/percolator/`
  - Konfigurasi workspace Cargo.toml agar includes percolator sebagai dependency
  - Verify: `anchor build`, `cargo test --lib`, `cargo fmt` semua sukses
  - Push initial scaffold ke branch `hypercolator-feature` di fork

  **Done kalau:**
  - `anchor build` sukses tanpa error
  - `cargo test --lib` pass
  - Folder `hypercolator/` terdapat di workspace dan di-push ke GitHub fork

  ---

  ### TASK #4 — Market Factory On-chain Program
  **Depends on:** Task #3

  **Yang dikerjakan:**
  - Tambah instruction `create_market(token_mint, stake_amount)` ke Anchor program
  - Buat account structs: MarketConfig (PDA), MarketRegistry, CreatorRecord
  - Validasi:
    - Minimum stake (~10 USD dalam SOL)
    - Max 3 markets per wallet
    - Token address harus valid SPL mint
  - Tier assignment logic:
    - Tier A (verified) → max leverage 50x
    - Tier B (mid-cap) → max leverage 20x
    - Tier C (unknown/pump.fun) → max leverage 5x
  - Register market ke MarketRegistry PDA
  - Unit tests: happy path, stake gagal, limit per-wallet gagal
  - `anchor build` + `cargo test --lib` sukses

  **Done kalau:**
  - Instruction `create_market` ada dan compiles
  - 3+ unit tests pass
  - Market terdaftar di MarketRegistry setelah create

  ---

  ### TASK #5 — Price Engine: TWAP & Anti-Manipulation
  **Depends on:** Task #4

  **Yang dikerjakan:**
  - Modul `price_engine.rs` di dalam program:
    - Baca harga spot dari AMM pool (Raydium/Orca compatible, x*y=k)
    - Accumulate TWAP over 30-menit window
    - Cek anti-manipulasi: min liquidity threshold, max deviation 20%
  - Instruction `update_twap` untuk keeper panggil tiap slot
  - Integrasi TWAP ke instruksi `liquidate`
  - Unit tests untuk: TWAP math, deviation rejection, min liquidity check

  **Done kalau:**
  - TWAP diupdate via `update_twap` instruction
  - Liquidation pakai harga TWAP yang sudah divalidasi
  - Unit tests semua pass

  ---

  ### TASK #6 — Market Lifecycle & Tier System
  **Depends on:** Task #4

  **Yang dikerjakan:**
  - Field `status` di MarketConfig: Inactive → Active → Expired
  - Instruction `activate_market`: aktif saat first trade/liquidity
  - Instruction `expire_market`: expired setelah 48h inactivity
    (tracking `last_activity_ts` di setiap trade)
  - Tier enforcement per-trade: cek leverage cap sesuai tier
  - Unit tests: semua state transitions, leverage cap enforcement

  **Done kalau:**
  - Market lifecycle berjalan sesuai state machine
  - Trading di market Expired ditolak
  - Leverage cap diterapkan sesuai tier

  ---

  ### TASK #7 — Frontend MVP (Next.js)
  **Depends on:** Task #4

  **Stack:**
  - Next.js 14 App Router
  - @solana/wallet-adapter (Phantom, Backpack, Solflare)
  - @coral-xyz/anchor untuk on-chain calls
  - Tailwind CSS

  **Halaman:**
  - `/` — Daftar semua market (harga, volume, status, tier badge)
  - `/markets/[address]` — Detail market + trading panel long/short
  - `/create` — Form buat market baru (paste token address)

  **Fitur:**
  - Connect/disconnect wallet
  - Buat market: paste token → preview tier → submit on-chain
  - Trade: pilih long/short, input size, leverage slider (dibatasi tier), submit
  - Orderbook display
  - Dark theme, feels like proper DEX

  **Done kalau:**
  - 3 halaman bisa diakses
  - Wallet connect berfungsi
  - Create market mengirim transaksi on-chain
  - Trading panel mengirim transaksi long/short

  ---

  ### TASK #8 — Keeper Bot & GitHub Automation
  **Depends on:** Task #5, Task #6

  **Keeper Bot (scripts/keeper/):**
  - Node.js polling loop tiap ~400ms (1 Solana slot)
  - Crank `update_twap` untuk semua active markets
  - Check posisi under-collateralized → panggil `liquidate`
  - Expire markets yang sudah 48h tidak aktif

  **GitHub Automation:**
  - Pakai GitHub App auth (App ID + PEM)
  - Auto-generate PR ke repo Toly dengan improvement dokumentasi
  - Open issue untuk feedback arsitektur
  - CI: .github/workflows/ci.yml (`cargo test`, `cargo fmt`, `cargo clippy`)

  **Done kalau:**
  - Keeper running dan bisa di-start via workflow
  - PR ter-draft di fork, siap di-submit ke upstream
  - CI pipeline hijau

  ---

  ## CATATAN PENTING

  1. **Percolator adalah pure Rust library** — bukan Solana program.
     Hypercolator mengambil LOGIKA risk engine-nya, lalu membungkusnya
     dalam Anchor instructions yang berjalan on-chain.

  2. **Pump.fun tokens tidak punya oracle** — solusinya adalah TWAP dari
     AMM pool langsung on-chain.

  3. **GitHub App tidak bisa di-ban** seperti PAT karena identitasnya
     terpisah dari akun personal.

  4. **Jangan push breaking changes ke upstream** — kontribusi hanya
     berupa dokumentasi, examples, dan modular extensions.
  