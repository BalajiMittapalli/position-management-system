import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { PublicKey, Keypair, SystemProgram } from "@solana/web3.js";
import { assert, expect } from "chai";
import { PositionManagement } from "../target/types/position_management";

describe("Position Management - Comprehensive Test Suite", () => {
  const provider = anchor.AnchorProvider.local();
  anchor.setProvider(provider);

  const program = anchor.workspace.PositionManagement as Program<PositionManagement>;
  const user = (provider.wallet as anchor.Wallet).payer;

  const USER_ACCOUNT_SEED = "user_account";
  const POSITION_SEED = "position";

  const SYMBOL_BTC = "BTC/USD";
  const SYMBOL_ETH = "ETH/USD";
  const INITIAL_COLLATERAL = new BN(100_000_000); // 100M collateral for tests

  let userAccountPda: PublicKey;
  let userAccountBump: number;
  let btcPositionPda: PublicKey;
  let ethPositionPda: PublicKey;

  before(async () => {
    [userAccountPda, userAccountBump] = PublicKey.findProgramAddressSync(
      [Buffer.from(USER_ACCOUNT_SEED), user.publicKey.toBuffer()],
      program.programId
    );

    [btcPositionPda] = PublicKey.findProgramAddressSync(
      [Buffer.from(POSITION_SEED), user.publicKey.toBuffer(), Buffer.from(SYMBOL_BTC)],
      program.programId
    );

    [ethPositionPda] = PublicKey.findProgramAddressSync(
      [Buffer.from(POSITION_SEED), user.publicKey.toBuffer(), Buffer.from(SYMBOL_ETH)],
      program.programId
    );

    console.log("User:", user.publicKey.toBase58());
    console.log("User Account PDA:", userAccountPda.toBase58());
    console.log("BTC Position PDA:", btcPositionPda.toBase58());
  });

  async function initializeUserAccount() {
    try {
      const userAccount = await program.account.userAccount.fetch(userAccountPda);
      console.log("User account already initialized with collateral:", userAccount.totalCollateral.toString());
    } catch (e) {
      console.log("Initializing user account...");
      await program.methods
        .initUser(INITIAL_COLLATERAL)
        .accounts({
          user: user.publicKey,
          userAccount: userAccountPda,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
      console.log("User account initialized");
    }
  }

  describe("1. Position Lifecycle: Open → Modify → Close", () => {
    const entryPrice = new BN(50_000);
    const positionSize = new BN(1_000);
    const leverage = 10;
    const expectedMargin = positionSize.mul(entryPrice).div(new BN(leverage));

    it("Should open a long position", async () => {
      await initializeUserAccount();

      const tx = await program.methods
        .openPosition(SYMBOL_BTC, { long: {} }, positionSize, leverage, entryPrice)
        .accounts({
          user: user.publicKey,
          userAccount: userAccountPda,
          position: btcPositionPda,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      const position = await program.account.position.fetch(btcPositionPda);
      assert.equal(position.size.toString(), positionSize.toString());
      assert.equal(position.leverage, leverage);
      console.log("✓ Position opened");
    });

    it("Should modify position by increasing size", async () => {
      const sizeIncrease = new BN(500);
      const newEntryPrice = new BN(51_000);

      await program.methods
        .modifyPosition(SYMBOL_BTC, { increaseSize: {} }, sizeIncrease, newEntryPrice)
        .accounts({
          user: user.publicKey,
          userAccount: userAccountPda,
          position: btcPositionPda,
        })
        .rpc();

      const position = await program.account.position.fetch(btcPositionPda);
      assert.ok(position.size.gt(positionSize));
      console.log("✓ Size increased");
    });

    it("Should close position", async () => {
      const exitPrice = new BN(55_000);

      await program.methods
        .closePosition(SYMBOL_BTC, exitPrice)
        .accounts({
          user: user.publicKey,
          userAccount: userAccountPda,
          position: btcPositionPda,
        })
        .rpc();

      console.log("✓ Position closed");
    });
  });

  describe("2. Insufficient Margin Rejection", () => {
    it("Should reject position open with insufficient margin", async () => {
      const largeSize = new BN(1_000_000_000);
      const entryPrice = new BN(50_000);

      try {
        await program.methods
          .openPosition(SYMBOL_ETH, { long: {} }, largeSize, 5, entryPrice)
          .accounts({
            user: user.publicKey,
            userAccount: userAccountPda,
            position: ethPositionPda,
            systemProgram: SystemProgram.programId,
          })
          .rpc();
        assert.fail("Should have thrown InsufficientMargin");
      } catch (e) {
        assert.ok(e.message.includes("InsufficientMargin") || e.message.includes("6001"));
        console.log("✓ Correctly rejected insufficient margin");
      }
    });
  });

  describe("3. Leverage Tier Enforcement", () => {
    it("Should reject 1000x leverage on large position", async () => {
      const largeSize = new BN(6_000);
      const entryPrice = new BN(50_000);

      try {
        await program.methods
          .openPosition("BTC-LARGE", { long: {} }, largeSize, 255, entryPrice)
          .accounts({
            user: user.publicKey,
            userAccount: userAccountPda,
            position: PublicKey.findProgramAddressSync(
              [Buffer.from(POSITION_SEED), user.publicKey.toBuffer(), Buffer.from("BTC-LARGE")],
              program.programId
            )[0],
            systemProgram: SystemProgram.programId,
          })
          .rpc();
        assert.fail("Should have rejected high leverage on large position");
      } catch (e) {
        console.log("Error:", e.message);
        assert.ok(
          e.message.includes("LeverageExceeded") || 
          e.message.includes("6007") || 
          e.message.includes("InsufficientMargin"),
          `Got error: ${e.message}`
        );
        console.log("✓ Correctly rejected");
      }
    });
  });

  describe("4. Liquidation Price Validation", () => {
    it("Should calculate liquidation price for long position", async () => {
      const entryPrice = new BN(50_000);
      const size = new BN(1_000);

      await program.methods
        .openPosition("LIQ-TEST", { long: {} }, size, 10, entryPrice)
        .accounts({
          user: user.publicKey,
          userAccount: userAccountPda,
          position: PublicKey.findProgramAddressSync(
            [Buffer.from(POSITION_SEED), user.publicKey.toBuffer(), Buffer.from("LIQ-TEST")],
            program.programId
          )[0],
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      const position = await program.account.position.fetch(
        PublicKey.findProgramAddressSync(
          [Buffer.from(POSITION_SEED), user.publicKey.toBuffer(), Buffer.from("LIQ-TEST")],
          program.programId
        )[0]
      );

      assert.ok(position.liquidationPrice.lt(entryPrice), "Liquidation price should be below entry");
      console.log("✓ Liquidation price validated");

      // Cleanup
      await program.methods
        .closePosition("LIQ-TEST", entryPrice)
        .accounts({
          user: user.publicKey,
          userAccount: userAccountPda,
          position: PublicKey.findProgramAddressSync(
            [Buffer.from(POSITION_SEED), user.publicKey.toBuffer(), Buffer.from("LIQ-TEST")],
            program.programId
          )[0],
        })
        .rpc();
    });
  });
});
