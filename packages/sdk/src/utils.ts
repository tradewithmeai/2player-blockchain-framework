import {
  Connection,
  PublicKey,
  TransactionInstruction,
  Keypair,
  Ed25519Program,
  ComputeBudgetProgram,
} from "@solana/web3.js";
import { getAssociatedTokenAddress, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  DEFAULT_COMPUTE_UNIT_LIMIT,
  DEFAULT_COMPUTE_UNIT_PRICE,
} from "./constants";
import { TxOptions } from "./types";
import * as crypto from "crypto";

/**
 * Get or create associated token account address
 */
export async function getOrCreateATA(
  connection: Connection,
  mint: PublicKey,
  owner: PublicKey,
  payer: PublicKey
): Promise<PublicKey> {
  const ata = await getAssociatedTokenAddress(mint, owner, false, TOKEN_PROGRAM_ID);

  const accountInfo = await connection.getAccountInfo(ata);

  // If account doesn't exist, it will be created via init_if_needed in the program
  return ata;
}

/**
 * Find PDA with seeds
 */
export function findProgramAddress(
  seeds: (Buffer | Uint8Array)[],
  programId: PublicKey
): [PublicKey, number] {
  const [address, bump] = PublicKey.findProgramAddressSync(seeds, programId);
  return [address, bump];
}

/**
 * Generate a random match ID
 */
export function generateMatchId(): Buffer {
  return crypto.randomBytes(32);
}

/**
 * Create compute budget instructions
 */
export function createComputeBudgetInstructions(options?: TxOptions): TransactionInstruction[] {
  const instructions: TransactionInstruction[] = [];

  const limit = options?.computeUnitLimit || DEFAULT_COMPUTE_UNIT_LIMIT;
  const price = options?.computeUnitPrice || DEFAULT_COMPUTE_UNIT_PRICE;

  instructions.push(ComputeBudgetProgram.setComputeUnitLimit({ units: limit }));

  if (price > 0) {
    instructions.push(
      ComputeBudgetProgram.setComputeUnitPrice({ microLamports: price })
    );
  }

  return instructions;
}

/**
 * Create ed25519 signature verification instructions
 *
 * This is used for realtime game settlement where both players must sign the result
 */
export function createEd25519VerifyInstructions(
  publicKey: PublicKey,
  message: Buffer,
  signature: Buffer
): TransactionInstruction {
  return Ed25519Program.createInstructionWithPublicKey({
    publicKey: publicKey.toBytes(),
    message,
    signature,
  });
}

/**
 * Create digest for realtime game settlement
 *
 * Both players must sign this digest off-chain
 */
export function createSettlementDigest(
  matchId: Buffer,
  winner: PublicKey,
  finalState: any,
  nonce: number,
  slot: number
): Buffer {
  const data = Buffer.concat([
    matchId,
    winner.toBuffer(),
    Buffer.from(JSON.stringify(finalState)),
    Buffer.from(nonce.toString()),
    Buffer.from(slot.toString()),
  ]);

  return crypto.createHash("sha256").update(data).digest();
}

/**
 * Convert SOL to lamports
 */
export function solToLamports(sol: number): number {
  return Math.floor(sol * 1_000_000_000);
}

/**
 * Convert lamports to SOL
 */
export function lamportsToSol(lamports: number): number {
  return lamports / 1_000_000_000;
}

/**
 * Convert SKILL (UI) to atomic units
 */
export function skillToAtomic(skill: number): number {
  return Math.floor(skill * 1_000_000);
}

/**
 * Convert atomic SKILL units to UI amount
 */
export function atomicToSkill(atomic: number): number {
  return atomic / 1_000_000;
}

/**
 * Sleep for specified milliseconds
 */
export function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Retry a function with exponential backoff
 */
export async function retryWithBackoff<T>(
  fn: () => Promise<T>,
  maxRetries: number = 4,
  baseDelay: number = 2000
): Promise<T> {
  for (let i = 0; i < maxRetries; i++) {
    try {
      return await fn();
    } catch (error) {
      if (i === maxRetries - 1) throw error;
      const delay = baseDelay * Math.pow(2, i);
      await sleep(delay);
    }
  }
  throw new Error("Max retries exceeded");
}
