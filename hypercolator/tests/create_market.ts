// eslint-disable-next-line @typescript-eslint/no-var-requires
const anchor = require("@coral-xyz/anchor");
// eslint-disable-next-line @typescript-eslint/no-var-requires
const idl = require("../target/idl/hypercolator.json");
// eslint-disable-next-line @typescript-eslint/no-var-requires
const spl = require("@solana/spl-token");
// eslint-disable-next-line @typescript-eslint/no-var-requires
const web3 = require("@solana/web3.js");
// eslint-disable-next-line @typescript-eslint/no-var-requires
const assert = require("assert");

const { AnchorProvider, BN, Program, setProvider } = anchor;
const { Keypair, PublicKey, SystemProgram, LAMPORTS_PER_SOL } = web3;
const { createMint, TOKEN_PROGRAM_ID } = spl;

// ---------------------------------------------------------------------------
// Constants — must match programs/hypercolator/src/state.rs exactly.
// ---------------------------------------------------------------------------
const MIN_STAKE_LAMPORTS = new BN(1_000_000);
const TRADING_FEE_BPS = 8;
const PROGRAM_ID = new PublicKey(
  "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS"
);

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function derivePdas(creator: any, tokenMint: any) {
  const [marketConfig] = PublicKey.findProgramAddressSync(
    [Buffer.from("market"), tokenMint.toBuffer()],
    PROGRAM_ID
  );
  const [marketRegistry] = PublicKey.findProgramAddressSync(
    [Buffer.from("registry")],
    PROGRAM_ID
  );
  const [creatorRecord] = PublicKey.findProgramAddressSync(
    [Buffer.from("creator"), creator.toBuffer()],
    PROGRAM_ID
  );
  return { marketConfig, marketRegistry, creatorRecord };
}

// ---------------------------------------------------------------------------
// Test suite
// ---------------------------------------------------------------------------

describe("create_market", () => {
  const provider = AnchorProvider.env();
  setProvider(provider);
  // In anchor 0.30.x the programId is taken from idl.address; no third arg.
  const program = new Program(idl, provider);

  // --------------------------------------------------------------------------
  // Test 1 — happy path: verify all on-chain state transitions
  // --------------------------------------------------------------------------
  it("happy path: creates market and populates all on-chain accounts", async () => {
    const creator = Keypair.generate();
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(
        creator.publicKey,
        0.1 * LAMPORTS_PER_SOL
      )
    );

    const mintAuthority = Keypair.generate();
    const tokenMint = await createMint(
      provider.connection,
      creator,
      mintAuthority.publicKey,
      null,
      6
    );

    const { marketConfig, marketRegistry, creatorRecord } = derivePdas(
      creator.publicKey,
      tokenMint
    );

    await program.methods
      .createMarket(MIN_STAKE_LAMPORTS)
      .accounts({
        creator: creator.publicKey,
        tokenMint,
        marketConfig,
        marketRegistry,
        creatorRecord,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([creator])
      .rpc();

    // Verify MarketConfig account fields.
    const config = await program.account.marketConfig.fetch(marketConfig);
    assert.ok(config.tokenMint.equals(tokenMint), "token_mint mismatch");
    assert.ok(config.creator.equals(creator.publicKey), "creator mismatch");
    assert.strictEqual(config.tier, 2, "unknown mint must be Tier C (u8=2)");
    assert.strictEqual(config.maxLeverageX, 5, "Tier C max leverage is 5x");
    assert.ok(config.stakeAmount.eq(MIN_STAKE_LAMPORTS), "stake_amount mismatch");
    assert.ok(config.insuranceFund.eqn(0), "insurance_fund must start at 0");
    assert.strictEqual(config.tradingFeeBps, TRADING_FEE_BPS, "fee_bps mismatch");
    assert.ok(config.isActive, "is_active must be true");

    // Verify MarketRegistry.
    const registry = await program.account.marketRegistry.fetch(marketRegistry);
    assert.strictEqual(registry.marketCount, 1, "registry market_count must be 1");
    assert.strictEqual(registry.markets.length, 1, "registry must list 1 market");
    assert.ok(registry.markets[0].equals(marketConfig), "registry must list the MarketConfig PDA");

    // Verify CreatorRecord.
    const record = await program.account.creatorRecord.fetch(creatorRecord);
    assert.ok(record.creator.equals(creator.publicKey), "record creator mismatch");
    assert.strictEqual(record.marketCount, 1, "creator market_count must be 1");
  });

  // --------------------------------------------------------------------------
  // Test 2 — stake too low → Anchor error 6000
  // --------------------------------------------------------------------------
  it("stake too low: rejected with error 6000 (StakeTooLow)", async () => {
    const creator = Keypair.generate();
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(
        creator.publicKey,
        0.1 * LAMPORTS_PER_SOL
      )
    );

    const mintAuthority = Keypair.generate();
    const tokenMint = await createMint(
      provider.connection,
      creator,
      mintAuthority.publicKey,
      null,
      6
    );

    const { marketConfig, marketRegistry, creatorRecord } = derivePdas(
      creator.publicKey,
      tokenMint
    );

    try {
      await program.methods
        .createMarket(MIN_STAKE_LAMPORTS.subn(1)) // one lamport short
        .accounts({
          creator: creator.publicKey,
          tokenMint,
          marketConfig,
          marketRegistry,
          creatorRecord,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([creator])
        .rpc();
      assert.fail("Transaction should have failed with StakeTooLow");
    } catch (err: any) {
      assert.strictEqual(
        err?.error?.errorCode?.number,
        6000,
        `Expected error 6000 (StakeTooLow), got: ${JSON.stringify(err?.error?.errorCode)}`
      );
    }
  });

  // --------------------------------------------------------------------------
  // Test 3 — wallet cap: fourth market fails with error 6001
  // --------------------------------------------------------------------------
  it("wallet cap: fourth market rejected with error 6001 (TooManyMarkets)", async () => {
    const creator = Keypair.generate();
    await provider.connection.confirmTransaction(
      await provider.connection.requestAirdrop(
        creator.publicKey,
        2 * LAMPORTS_PER_SOL
      )
    );
    const mintAuthority = Keypair.generate();

    // Open three markets to fill the per-creator cap.
    for (let i = 0; i < 3; i++) {
      const tokenMint = await createMint(
        provider.connection,
        creator,
        mintAuthority.publicKey,
        null,
        6
      );
      const { marketConfig, marketRegistry, creatorRecord } = derivePdas(
        creator.publicKey,
        tokenMint
      );
      await program.methods
        .createMarket(MIN_STAKE_LAMPORTS)
        .accounts({
          creator: creator.publicKey,
          tokenMint,
          marketConfig,
          marketRegistry,
          creatorRecord,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([creator])
        .rpc();
    }

    // Fourth market must be rejected.
    const fourthMint = await createMint(
      provider.connection,
      creator,
      mintAuthority.publicKey,
      null,
      6
    );
    const { marketConfig, marketRegistry, creatorRecord } = derivePdas(
      creator.publicKey,
      fourthMint
    );

    try {
      await program.methods
        .createMarket(MIN_STAKE_LAMPORTS)
        .accounts({
          creator: creator.publicKey,
          tokenMint: fourthMint,
          marketConfig,
          marketRegistry,
          creatorRecord,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([creator])
        .rpc();
      assert.fail("Transaction should have failed with TooManyMarkets");
    } catch (err: any) {
      assert.strictEqual(
        err?.error?.errorCode?.number,
        6001,
        `Expected error 6001 (TooManyMarkets), got: ${JSON.stringify(err?.error?.errorCode)}`
      );
    }
  });
});
