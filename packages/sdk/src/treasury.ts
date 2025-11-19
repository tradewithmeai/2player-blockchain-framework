import {
  Connection,
  PublicKey,
  TransactionInstruction,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
} from "@solana/web3.js";
import { Program, AnchorProvider, BN } from "@coral-xyz/anchor";
import { getAssociatedTokenAddress } from "@solana/spl-token";
import {
  SKILL_TREASURY_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
} from "./constants";
import { findProgramAddress, createComputeBudgetInstructions } from "./utils";
import { TreasuryConfig, TxOptions } from "./types";

export class TreasuryClient {
  constructor(
    public connection: Connection,
    public programId: PublicKey = SKILL_TREASURY_PROGRAM_ID
  ) {}

  /**
   * Get treasury config PDA
   */
  getConfigAddress(): [PublicKey, number] {
    return findProgramAddress([Buffer.from("config")], this.programId);
  }

  /**
   * Get treasury SOL vault PDA
   */
  getTreasurySolVaultAddress(): [PublicKey, number] {
    return findProgramAddress([Buffer.from("treasury_sol_vault")], this.programId);
  }

  /**
   * Fetch treasury config
   */
  async getConfig(): Promise<TreasuryConfig | null> {
    const [configPda] = this.getConfigAddress();
    const accountInfo = await this.connection.getAccountInfo(configPda);

    if (!accountInfo) return null;

    // Manually decode or use Anchor Program
    // For now, returning null - implement full decoding with generated IDL
    return null;
  }

  /**
   * Build initialize instruction
   */
  async buildInitialize(
    admin: PublicKey,
    skillMint: PublicKey,
    rate: BN,
    feeBps: number
  ): Promise<TransactionInstruction[]> {
    const [config] = this.getConfigAddress();
    const [treasurySolVault] = this.getTreasurySolVaultAddress();

    // This would use the generated Anchor IDL
    // For now, providing the structure
    // Actual implementation requires loading the IDL

    return [];
  }

  /**
   * Build deposit SOL and mint SKILL instruction
   */
  async buildDepositSolAndMintSkill(
    user: PublicKey,
    skillMint: PublicKey,
    lamports: BN,
    options?: TxOptions
  ): Promise<TransactionInstruction[]> {
    const instructions: TransactionInstruction[] = [];

    // Add compute budget
    if (options) {
      instructions.push(...createComputeBudgetInstructions(options));
    }

    const [config] = this.getConfigAddress();
    const [treasurySolVault] = this.getTreasurySolVaultAddress();
    const userSkillAta = await getAssociatedTokenAddress(
      skillMint,
      user,
      false,
      TOKEN_PROGRAM_ID
    );

    // Build instruction using Anchor
    // Requires generated IDL

    return instructions;
  }

  /**
   * Build redeem SKILL for SOL instruction
   */
  async buildRedeemSkillForSol(
    user: PublicKey,
    skillMint: PublicKey,
    amount: BN,
    options?: TxOptions
  ): Promise<TransactionInstruction[]> {
    const instructions: TransactionInstruction[] = [];

    // Add compute budget
    if (options) {
      instructions.push(...createComputeBudgetInstructions(options));
    }

    const [config] = this.getConfigAddress();
    const [treasurySolVault] = this.getTreasurySolVaultAddress();
    const userSkillAta = await getAssociatedTokenAddress(
      skillMint,
      user,
      false,
      TOKEN_PROGRAM_ID
    );

    // Build instruction using Anchor
    // Requires generated IDL

    return instructions;
  }

  /**
   * Calculate SKILL amount from SOL
   */
  calculateSkillFromSol(lamports: BN, rate: BN): BN {
    return lamports.mul(rate).div(new BN(1_000_000_000));
  }

  /**
   * Calculate SOL amount from SKILL
   */
  calculateSolFromSkill(skillAmount: BN, rate: BN): BN {
    return skillAmount.mul(new BN(1_000_000_000)).div(rate);
  }
}
