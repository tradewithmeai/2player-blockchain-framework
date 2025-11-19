import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { PublicKey, Keypair, SystemProgram } from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  createMint,
  getAssociatedTokenAddress,
  getOrCreateAssociatedTokenAccount,
} from "@solana/spl-token";
import { assert } from "chai";
import { SkillTreasury } from "../target/types/skill_treasury";

describe("skill_treasury", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.SkillTreasury as Program<SkillTreasury>;
  const admin = provider.wallet.publicKey;

  let skillMint: PublicKey;
  let configPda: PublicKey;
  let treasurySolVault: PublicKey;

  const RATE = new anchor.BN(1000); // 1 SOL = 1000 SKILL
  const FEE_BPS = 100; // 1%

  before(async () => {
    // Create SKILL mint
    skillMint = await createMint(
      provider.connection,
      provider.wallet.payer,
      admin,
      admin,
      6 // decimals
    );

    // Derive PDAs
    [configPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("config")],
      program.programId
    );

    [treasurySolVault] = PublicKey.findProgramAddressSync(
      [Buffer.from("treasury_sol_vault")],
      program.programId
    );
  });

  it("Initializes the treasury", async () => {
    const tx = await program.methods
      .initialize(RATE, FEE_BPS)
      .accounts({
        config: configPda,
        treasurySolVault,
        skillMint,
        admin,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    console.log("Initialize tx:", tx);

    const config = await program.account.config.fetch(configPda);
    assert.equal(config.rate.toString(), RATE.toString());
    assert.equal(config.feeBps, FEE_BPS);
    assert.equal(config.admin.toString(), admin.toString());
  });

  it("Deposits SOL and mints SKILL", async () => {
    const user = Keypair.generate();

    // Airdrop SOL to user
    const airdropSig = await provider.connection.requestAirdrop(
      user.publicKey,
      2 * anchor.web3.LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(airdropSig);

    const userSkillAta = await getAssociatedTokenAddress(
      skillMint,
      user.publicKey
    );

    const depositAmount = new anchor.BN(1 * anchor.web3.LAMPORTS_PER_SOL);

    const tx = await program.methods
      .depositSolAndMintSkill(depositAmount)
      .accounts({
        config: configPda,
        treasurySolVault,
        skillMint,
        userSkillAta,
        user: user.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([user])
      .rpc();

    console.log("Deposit tx:", tx);

    // Verify SKILL was minted
    const userSkillAccount = await getOrCreateAssociatedTokenAccount(
      provider.connection,
      provider.wallet.payer,
      skillMint,
      user.publicKey
    );

    const expectedSkill = depositAmount.mul(RATE).div(
      new anchor.BN(anchor.web3.LAMPORTS_PER_SOL)
    );

    assert.equal(
      userSkillAccount.amount.toString(),
      expectedSkill.toString()
    );
  });

  it("Redeems SKILL for SOL", async () => {
    const user = Keypair.generate();

    // Setup: deposit first
    const airdropSig = await provider.connection.requestAirdrop(
      user.publicKey,
      2 * anchor.web3.LAMPORTS_PER_SOL
    );
    await provider.connection.confirmTransaction(airdropSig);

    const userSkillAta = await getAssociatedTokenAddress(
      skillMint,
      user.publicKey
    );

    const depositAmount = new anchor.BN(1 * anchor.web3.LAMPORTS_PER_SOL);

    await program.methods
      .depositSolAndMintSkill(depositAmount)
      .accounts({
        config: configPda,
        treasurySolVault,
        skillMint,
        userSkillAta,
        user: user.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .signers([user])
      .rpc();

    // Now redeem half
    const redeemAmount = new anchor.BN(500 * 1_000_000); // 500 SKILL

    const userBalanceBefore = await provider.connection.getBalance(
      user.publicKey
    );

    const tx = await program.methods
      .redeemSkillForSol(redeemAmount)
      .accounts({
        config: configPda,
        treasurySolVault,
        skillMint,
        userSkillAta,
        user: user.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([user])
      .rpc();

    console.log("Redeem tx:", tx);

    const userBalanceAfter = await provider.connection.getBalance(
      user.publicKey
    );

    // User should have more SOL (minus transaction fees)
    assert.isTrue(userBalanceAfter > userBalanceBefore);
  });
});
