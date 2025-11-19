import { Connection, PublicKey, TransactionInstruction } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
import { getAssociatedTokenAddress } from "@solana/spl-token";
import {
  TTT_ONCHAIN_PROGRAM_ID,
  SKILL_ESCROW_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
} from "./constants";
import { findProgramAddress, createComputeBudgetInstructions } from "./utils";
import { Game, TxOptions } from "./types";

export class TicTacToeClient {
  constructor(
    public connection: Connection,
    public programId: PublicKey = TTT_ONCHAIN_PROGRAM_ID,
    public escrowProgramId: PublicKey = SKILL_ESCROW_PROGRAM_ID
  ) {}

  /**
   * Get game PDA
   */
  getGameAddress(matchId: Buffer): [PublicKey, number] {
    return findProgramAddress([Buffer.from("game"), matchId], this.programId);
  }

  /**
   * Fetch game data
   */
  async getGame(matchId: Buffer): Promise<Game | null> {
    const [gamePda] = this.getGameAddress(matchId);
    const accountInfo = await this.connection.getAccountInfo(gamePda);

    if (!accountInfo) return null;

    // Decode using Anchor IDL
    return null;
  }

  /**
   * Build init game instruction
   */
  async buildInitGame(
    payer: PublicKey,
    matchId: Buffer,
    timeoutSlots: BN,
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
   * Build play move instruction
   */
  async buildPlay(
    player: PublicKey,
    matchId: Buffer,
    position: number,
    options?: TxOptions
  ): Promise<TransactionInstruction[]> {
    const instructions: TransactionInstruction[] = [];

    if (options) {
      instructions.push(...createComputeBudgetInstructions(options));
    }

    const [gamePda] = this.getGameAddress(matchId);

    // Build instruction with Anchor IDL
    return instructions;
  }

  /**
   * Build resolve game instruction
   */
  async buildResolveIfComplete(
    matchId: Buffer,
    winner: PublicKey,
    skillMint: PublicKey,
    options?: TxOptions
  ): Promise<TransactionInstruction[]> {
    const instructions: TransactionInstruction[] = [];

    if (options) {
      instructions.push(...createComputeBudgetInstructions(options));
    }

    const [gamePda] = this.getGameAddress(matchId);

    // Get escrow PDAs
    const [matchPda] = findProgramAddress(
      [Buffer.from("match"), matchId],
      this.escrowProgramId
    );
    const [escrowVault] = findProgramAddress(
      [Buffer.from("escrow_vault"), matchId],
      this.escrowProgramId
    );
    const [feeVault] = findProgramAddress(
      [Buffer.from("fee_vault")],
      this.escrowProgramId
    );

    const winnerAta = await getAssociatedTokenAddress(
      skillMint,
      winner,
      false,
      TOKEN_PROGRAM_ID
    );

    // Build instruction with Anchor IDL
    // This will CPI to skill_escrow.settle_onchain
    return instructions;
  }

  /**
   * Build timeout claim instruction
   */
  async buildTimeout(
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

  /**
   * Check if position is valid
   */
  isValidPosition(position: number): boolean {
    return position >= 0 && position <= 8;
  }

  /**
   * Check for winner in a board state
   */
  checkWinner(board: number[]): number | null {
    const lines = [
      [0, 1, 2],
      [3, 4, 5],
      [6, 7, 8], // Rows
      [0, 3, 6],
      [1, 4, 7],
      [2, 5, 8], // Columns
      [0, 4, 8],
      [2, 4, 6], // Diagonals
    ];

    for (const [a, b, c] of lines) {
      if (board[a] !== 0 && board[a] === board[b] && board[b] === board[c]) {
        return board[a];
      }
    }

    return null;
  }

  /**
   * Check if board is full (draw)
   */
  isBoardFull(board: number[]): boolean {
    return board.every((cell) => cell !== 0);
  }

  /**
   * Get available moves
   */
  getAvailableMoves(board: number[]): number[] {
    return board.map((cell, i) => (cell === 0 ? i : -1)).filter((i) => i !== -1);
  }
}
