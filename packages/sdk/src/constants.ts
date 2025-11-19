import { PublicKey } from "@solana/web3.js";

// Program IDs (update these after deployment)
export const SKILL_TREASURY_PROGRAM_ID = new PublicKey(
  "SkTry9999999999999999999999999999999999999"
);

export const SKILL_ESCROW_PROGRAM_ID = new PublicKey(
  "SkEsc9999999999999999999999999999999999999"
);

export const TTT_ONCHAIN_PROGRAM_ID = new PublicKey(
  "TicTa9999999999999999999999999999999999999"
);

// SPL Token Program (classic, not Token-2022)
export const TOKEN_PROGRAM_ID = new PublicKey(
  "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
);

export const ASSOCIATED_TOKEN_PROGRAM_ID = new PublicKey(
  "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
);

export const SYSTEM_PROGRAM_ID = new PublicKey("11111111111111111111111111111111");

// Ed25519 program for signature verification
export const ED25519_PROGRAM_ID = new PublicKey(
  "Ed25519SigVerify111111111111111111111111111"
);

// Compute budget defaults
export const DEFAULT_COMPUTE_UNIT_LIMIT = 200_000;
export const DEFAULT_COMPUTE_UNIT_PRICE = 1; // micro-lamports

// Constants
export const SKILL_DECIMALS = 6;
export const LAMPORTS_PER_SOL = 1_000_000_000;
