# HYPERCOLATOR - MASTER PLAN

> Decentralized Perpetual Futures Exchange on Solana
> Based on Percolator by Anatoly Yakovenko (Toly)

--

## RINGKASAN PROYEK

Hypercolator adalah fork dari Percolator yang dikembangkan menjadi:
- DEX perpetual futures yang permissionless (siapapun bisa buat market)
- Mendukung token pump.fun tanpa oracle eksternal
- Sistem harga TWAP berbasis AMM pool on-chain
- Self-funded insurance model - tim TIDAK perlu sediain modal awal
- Frontend Next.js dengan Solana Wallet Adapter
- Keeper bot otomatis untuk likuidasi & funding

--

## MODEL KEUANGAN - SELF-FUNDED INSURANCE

### Konsep Inti
Tim TIDAK menyediakan dana insurance. Dana cadangan tumbuh otomatis dari fee setiap transaksi trading.

### Alur Dana
```
Setiap trade terjadi
        │
        ▼
Fee 0.1% dipotong otomatis dari posisi
        │
   ┌────┴────┐
   ▼         ▼
 0.08%     0.02%
Insurance  Protocol
  Fund      Fee
(on-chain) (ke tim)
```

### Fee Structure
| Jenis Fee | Besaran | Ke mana | Tujuan |
|--|--|--|--|
| Insurance fee | 0.08% per trade | Insurance Fund on-chain | Backup bad debt |
| Protocol fee | 0.02% per trade | Wallet tim | Pendapatan operasional |
| Market creation stake | ~$10 USDC | Insurance Fund market itu | Seed per-market |

### Cara Bad Debt Ditangani (urutan prioritas)
```
1. Keeper bot likuidasi posisi sebelum modal habis  ← paling sering berhasil
        │ (kalau gagal/telat)
        ▼
2. ADL - posisi paling untung di sisi berlawanan
   dipaksa tutup sebagian untuk nutupin lubang
        │ (kalau masih kurang)
        ▼
3. Insurance Fund (dari fee yang terkumpul) nutupin sisanya
        │ (kalau insurance fund kosong - sangat jarang)
        ▼
4. Socialized loss - haircut kecil ke semua pemenang di market itu
```

### Proyeksi Insurance Fund
| Volume Harian | Fee Masuk/Hari | Fund Setelah 1 Bulan |
|--|--|--|
| $10,000 | $8 | ~$240 |
| $100,000 | $80 | ~$2,400 |
| $1,000,000 | $800 | ~$24,000 |

**Semakin ramai exchange → insurance fund makin tebal otomatis.**

### Biaya Awal Tim (satu kali)
| Item | Estimasi | Keterangan |
|--|--|--|
| Deploy program Solana | 2–5 SOL | Sekali bayar |
| VPS untuk keeper bot | ~$10–20/bulan | Server murah |
| **Total modal awal** | **< $1,000** | Tidak ada dana insurance dari tim |

--

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
|--|--|
| `GITHUB_APP_ID` | Angka App ID dari langkah 7 |
| `GITHUB_APP_PRIVATE_KEY` | Isi seluruh file .pem (paste langsung) |
| `GITHUB_APP_INSTALLATION_ID` | Angka Installation ID dari langkah 9 |
| `GITHUB_USERNAME` | Username GitHub kamu |

--

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
Task #4: Market Factory + Fee & Insurance System
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

--

## DETAIL SETIAP TASK

--

### TASK #1 - GitHub App Setup & Repository Bootstrap
**Depends on:** nothing (mulai dari sini)

**Yang dikerjakan:**
- Buat helper module TypeScript untuk GitHub App authentication
  (JWT generation dari App ID + PEM → exchange ke Installation Token)
- Fork repo `aeyakovenko/percolator` ke akun GitHub kamu via GitHub API
- Clone fork ke workspace: `hypercolator/`
- Buat branch `hypercolator-feature` di fork
- Open Issue #1 di repo Toly: "Architecture feedback request - Hypercolator extension"
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

--

### TASK #2 - Percolator Research & Architecture
**Depends on:** Task #1

**Yang dikerjakan:**
- Baca spec.md (~112KB) dari repo Toly
- Baca seluruh source: percolator.rs, i128.rs, wide_math.rs
- Identifikasi semua public types, enums, function signatures
- Tulis `docs/percolator-architecture.md` berisi:
  - Diagram arsitektur (text-based)
  - Semua modul dan fungsinya
  - Cara liquidation bekerja (ADL system)
  - Cara funding rate dihitung
  - Cara PnL warmup bekerja
  - Batasan: ini library pure Rust, BUKAN Solana program
  - Adaptation plan: apa yang perlu dibangun from scratch untuk Hypercolator

**Done kalau:**
- File `docs/percolator-architecture.md` ada dan lengkap
- Ada seksi jelas tentang "apa yang harus dibangun Hypercolator dari nol"

--

### TASK #3 - Rust/Solana Toolchain & Anchor Scaffold
**Depends on:** Task #2

**Yang dikerjakan:**
- Install Rust stable + Solana CLI + Anchor framework via Nix/replit.nix
- Inisialisasi Anchor project: `anchor init hypercolator` di dalam folder `hypercolator/`
- Vendor Percolator library sebagai local Cargo crate: `hypercolator/crates/percolator/`
- Konfigurasi workspace Cargo.toml agar includes percolator sebagai dependency
- Verify: `anchor build`, `cargo test -lib`, `cargo fmt` semua sukses
- Push initial scaffold ke branch `hypercolator-feature` di fork

**Done kalau:**
- `anchor build` sukses tanpa error
- `cargo test -lib` pass
- Folder `hypercolator/` terdapat di workspace dan di-push ke GitHub fork

--

### TASK #4 - Market Factory + Fee & Insurance System
**Depends on:** Task #3

**Yang dikerjakan:**

**Market Factory:**
- Instruction `create_market(token_mint, stake_amount)`
- Account structs: MarketConfig (PDA), MarketRegistry, CreatorRecord
- Validasi: minimum stake ~$10, max 3 markets per wallet, token valid
- Tier assignment: Tier A (50x), Tier B (20x), Tier C pump.fun (5x)

**Fee & Insurance System (BARU - tidak ada modal dari tim):**
- Account `InsuranceFund` on-chain per-market: menyimpan fee yang terkumpul
- Setiap trade instruction:
  - Potong 0.08% → masuk `InsuranceFund` account
  - Potong 0.02% → masuk `ProtocolFeeVault` (dompet tim)
- Market creation stake (~$10) → masuk `InsuranceFund` market itu sebagai seed
- Instruction `pay_from_insurance(amount)` hanya bisa dipanggil program sendiri saat bad debt
- Saldo insurance fund transparan - siapapun bisa lihat on-chain

**Done kalau:**
- `create_market` berhasil dan market terdaftar
- Setiap trade otomatis potong fee dan distribusi ke fund yang benar
- Insurance fund saldo bisa dibaca on-chain
- 5+ unit tests pass

--

### TASK #5 - Price Engine: TWAP & Anti-Manipulation
**Depends on:** Task #4

**Yang dikerjakan:**
- Modul `price_engine.rs`:
  - Baca harga spot dari AMM pool (Raydium/Orca, x*y=k formula)
  - Accumulate TWAP over 30-menit window
  - Anti-manipulasi: min liquidity threshold, max deviation 20%
- Instruction `update_twap` untuk keeper panggil tiap slot
- Integrasi TWAP ke instruksi `liquidate`
- Unit tests: TWAP math, deviation rejection, min liquidity

**Done kalau:**
- TWAP diupdate via `update_twap` instruction
- Liquidation pakai harga TWAP yang sudah divalidasi
- Unit tests semua pass

--

### TASK #6 - Market Lifecycle & Tier System
**Depends on:** Task #4

**Yang dikerjakan:**
- Field `status` di MarketConfig: Inactive → Active → Expired
- Instruction `activate_market`: aktif saat first trade
- Instruction `expire_market`: expired setelah 48h inactivity
- Field `last_activity_ts` diupdate di setiap trade
- Tier enforcement per-trade: leverage cap sesuai tier
- Unit tests: semua state transitions, leverage cap enforcement

**Done kalau:**
- Market lifecycle berjalan sesuai state machine
- Trading di market Expired ditolak
- Leverage cap diterapkan sesuai tier

--

### TASK #7 - Frontend MVP (Next.js)
**Depends on:** Task #4

**Stack:**
- Next.js 14 App Router
- @solana/wallet-adapter (Phantom, Backpack, Solflare)
- @coral-xyz/anchor untuk on-chain calls
- Tailwind CSS

**Halaman:**
- `/` - Daftar semua market (harga, volume, status, tier badge)
- `/markets/[address]` - Detail market + trading panel long/short
- `/create` - Form buat market baru (paste token address)

**Fitur:**
- Connect/disconnect wallet
- Buat market: paste token → preview tier/stake → submit on-chain
- Trade: pilih long/short, size, leverage slider (dibatasi tier), submit
- Tampilkan saldo Insurance Fund per-market (transparan)
- Orderbook display
- Dark theme, tampilan seperti DEX profesional

**Done kalau:**
- 3 halaman bisa diakses
- Wallet connect berfungsi
- Create market mengirim transaksi on-chain
- Trading panel mengirim transaksi long/short

--

### TASK #8 - Keeper Bot & GitHub Automation
**Depends on:** Task #5, Task #6

**Keeper Bot (scripts/keeper/):**
- Node.js polling loop tiap ~400ms (1 Solana slot)
- Crank `update_twap` untuk semua active markets
- Check posisi under-collateralized → panggil `liquidate`
  (kerugian dibayar dari insurance fund dulu, baru ADL)
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

--

## CATATAN PENTING

1. **Tim tidak perlu sediain dana insurance** - tumbuh otomatis dari fee setiap trade (0.08%).

2. **Percolator adalah pure Rust library** - bukan Solana program.
   Hypercolator mengambil logika risk engine-nya, lalu membungkusnya
   dalam Anchor instructions yang berjalan on-chain.

3. **Pump.fun tokens tidak punya oracle** - solusinya TWAP dari AMM pool on-chain.

4. **GitHub App tidak bisa di-ban** seperti PAT karena identitasnya
   terpisah dari akun personal.

5. **Jangan push breaking changes ke upstream** - kontribusi hanya
   berupa dokumentasi, examples, dan modular extensions.

6. **Insurance fund transparan** - semua orang bisa lihat saldonya on-chain,
   membangun kepercayaan user tanpa tim harus buktikan apa-apa.
