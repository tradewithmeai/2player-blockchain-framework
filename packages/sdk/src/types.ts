import { PublicKey } from "@solana/web3.js";
import BN from "bn.js";

// Match status enum
export enum MatchStatus {
  Created = 0,
  Active = 1,
  Settled = 2,
  Cancelled = 3,
}

// Game mode enum
export enum GameMode {
  TurnBased = 0,
  Realtime = 1,
}

// Game status enum
export enum GameStatus {
  Active = 0,
  Finished = 1,
}

// Treasury types
export interface TreasuryConfig {
  rate: BN;
  feeBps: number;
  admin: PublicKey;
  skillMint: PublicKey;
  tokenProgram: PublicKey;
  treasuryBump: number;
}

// Escrow types
export interface EscrowConfig {
  admin: PublicKey;
  feeBps: number;
  skillMint: PublicKey;
  bump: number;
}

export interface Match {
  matchId: Buffer;
  player1: PublicKey;
  player2: PublicKey;
  stake: BN;
  skillMint: PublicKey;
  status: MatchStatus;
  mode: GameMode;
  expirySlot: BN;
  feeBps: number;
  player1Funded: boolean;
  player2Funded: boolean;
  bump: number;
}

// Tic-Tac-Toe types
export interface Game {
  matchId: Buffer;
  matchPda: PublicKey;
  playerX: PublicKey;
  playerO: PublicKey;
  board: number[];
  currentTurn: number; // 1 = X, 2 = O
  winner: PublicKey | null;
  status: GameStatus;
  moveCount: number;
  deadlineSlot: BN;
  timeoutSlots: BN;
  bump: number;
}

// Client options
export interface ClientOptions {
  treasuryProgramId?: PublicKey;
  escrowProgramId?: PublicKey;
  tttProgramId?: PublicKey;
}

// Transaction options
export interface TxOptions {
  computeUnitLimit?: number;
  computeUnitPrice?: number;
}
