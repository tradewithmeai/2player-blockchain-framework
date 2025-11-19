import {
  Connection,
  PublicKey,
  TransactionInstruction,
  SYSVAR_INSTRUCTIONS_PUBKEY,
} from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
import { getAssociatedTokenAddress } from "@solana/spl-token";
import {
  SKILL_ESCROW_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
} from "./constants";
import {
  findProgramAddress,
  createComputeBudgetInstructions,
  createEd25519VerifyInstructions,
} from "./utils";
import { Match, GameMode, TxOptions } from "./types";

export class EscrowClient {
  constructor(
    public connection: Connection,
    public programId: PublicKey = SKILL_ESCROW_PROGRAM_ID
  ) {}

  /**
   * Get escrow config PDA
   */
  getConfigAddress(): [PublicKey, number] {
    return findProgramAddress([Buffer.from("config")], this.programId);
  }

  /**
   * Get match PDA
   */
  getMatchAddress(matchId: Buffer): [PublicKey, number] {
    return findProgramAddress([Buffer.from("match"), matchId], this.programId);
  }

  /**
   * Get escrow vault PDA for a match
   */
  getEscrowVaultAddress(matchId: Buffer): [PublicKey, number] {
    return findProgramAddress([Buffer.from("escrow_vault"), matchId], this.programId);
  }

  /**
   * Get fee vault PDA
   */
  getFeeVaultAddress(): [PublicKey, number] {
    return findProgramAddress([Buffer.from("fee_vault")], this.programId);
  }

  /**
   * Fetch match data
   */
  async getMatch(matchId: Buffer): Promise<Match | null> {
    const [matchPda] = this.getMatchAddress(matchId);
    const accountInfo = await this.connection.getAccountInfo(matchPda);

    if (!accountInfo) return null;

    // Decode using Anchor IDL
    // For now, returning null
    return null;
  }

  /**
   * Build create match instruction
   */
  async buildCreateMatch(
    player1: PublicKey,
    matchId: Buffer,
    stake: BN,
    mode: GameMode,
    expirySlots: BN,
    skillMint: PublicKey,
    options?: TxOptions
  ): Promise<TransactionInstruction[]> {
    const instructions: TransactionInstruction[] = [];

    if (options) {
      instructions.push(...createComputeBudgetInstructions(options));
    }

    // Build instruction with Anchor IDL
    return instructions;
  }

  /**
   * Build join match instruction
   */
  async buildJoinMatch(
    player2: PublicKey,
    matchId: Buffer,
    options?: TxOptions
  ): Promise<TransactionInstruction[]> {
    const instructions: TransactionInstruction[] = [];

    if (options) {
      instructions.push(...createComputeBudgetInstructions(options));
    }

    // Build instruction with Anchor IDL
    return instructions;
  }

  /**
   * Build fund match instruction
   */
  async buildFund(
    player: PublicKey,
    matchId: Buffer,
    skillMint: PublicKey,
    options?: TxOptions
  ): Promise<TransactionInstruction[]> {
    const instructions: TransactionInstruction[] = [];

    if (options) {
      instructions.push(...createComputeBudgetInstructions(options));
    }

    const [matchPda] = this.getMatchAddress(matchId);
    const [escrowVault] = this.getEscrowVaultAddress(matchId);
    const playerSkillAta = await getAssociatedTokenAddress(
      skillMint,
      player,
      false,
      TOKEN_PROGRAM_ID
    );

    // Build instruction with Anchor IDL
    return instructions;
  }

  /**
   * Build start match instruction
   */
  async buildStartIfBothFunded(
    matchId: Buffer,
    options?: TxOptions
  ): Promise<TransactionInstruction[]> {
    const instructions: TransactionInstruction[] = [];

    if (options) {
      instructions.push(...createComputeBudgetInstructions(options));
    }

    // Build instruction with Anchor IDL
    return instructions;
  }

  /**
   * Build settle with signatures instruction (for realtime games)
   *
   * Requires ed25519 verify instructions for both players
   */
  async buildSettleWithSignatures(
    matchId: Buffer,
    winner: PublicKey,
    digest: Buffer,
    player1Signature: Buffer,
    player2Signature: Buffer,
    player1: PublicKey,
    player2: PublicKey,
    skillMint: PublicKey,
    options?: TxOptions
  ): Promise<TransactionInstruction[]> {
    const instructions: TransactionInstruction[] = [];

    if (options) {
      instructions.push(...createComputeBudgetInstructions(options));
    }

    // Add ed25519 verify instructions BEFORE the settle instruction
    instructions.push(createEd25519VerifyInstructions(player1, digest, player1Signature));
    instructions.push(createEd25519VerifyInstructions(player2, digest, player2Signature));

    const [matchPda] = this.getMatchAddress(matchId);
    const [escrowVault] = this.getEscrowVaultAddress(matchId);
    const [feeVault] = this.getFeeVaultAddress();
    const winnerAta = await getAssociatedTokenAddress(
      skillMint,
      winner,
      false,
      TOKEN_PROGRAM_ID
    );

    // Build instruction with Anchor IDL
    // Must include SYSVAR_INSTRUCTIONS_PUBKEY

    return instructions;
  }

  /**
   * Build cancel match instruction
   */
  async buildCancelIfExpired(
    matchId: Buffer,
    skillMint: PublicKey,
    player1: PublicKey,
    player2: PublicKey,
    options?: TxOptions
  ): Promise<TransactionInstruction[]> {
    const instructions: TransactionInstruction[] = [];

    if (options) {
      instructions.push(...createComputeBudgetInstructions(options));
    }

    // Build instruction with Anchor IDL
    return instructions;
  }

  /**
   * Build claim timeout instruction
   */
  async buildClaimTimeout(
    claimant: PublicKey,
    matchId: Buffer,
    skillMint: PublicKey,
    options?: TxOptions
  ): Promise<TransactionInstruction[]> {
    const instructions: TransactionInstruction[] = [];

    if (options) {
      instructions.push(...createComputeBudgetInstructions(options));
    }

    // Build instruction with Anchor IDL
    return instructions;
  }
}
