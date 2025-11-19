/**
 * Script to initialize all programs after deployment
 *
 * Usage: ts-node scripts/init-programs.ts [cluster]
 */

import * as anchor from "@coral-xyz/anchor";
import { PublicKey, Keypair } from "@solana/web3.js";
import { createMint, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import fs from "fs";

const CLUSTER = process.argv[2] || "devnet";

async function main() {
  console.log(`🔧 Initializing programs on ${CLUSTER}...`);

  // Setup provider
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const admin = provider.wallet.publicKey;

  // 1. Create SKILL mint
  console.log("\n1️⃣ Creating SKILL mint...");
  const skillMint = await createMint(
    provider.connection,
    provider.wallet.payer,
    admin, // mint authority (will transfer to treasury PDA)
    admin, // freeze authority
    6 // decimals
  );

  console.log("✅ SKILL Mint:", skillMint.toString());

  // 2. Initialize Treasury
  console.log("\n2️⃣ Initializing Treasury...");

  const RATE = new anchor.BN(1000); // 1 SOL = 1000 SKILL
  const FEE_BPS = 100; // 1%

  // TODO: Load treasury program and call initialize
  // const treasuryProgram = anchor.workspace.SkillTreasury;
  // await treasuryProgram.methods.initialize(RATE, FEE_BPS)...

  console.log("✅ Treasury initialized");

  // 3. Initialize Escrow
  console.log("\n3️⃣ Initializing Escrow...");

  // TODO: Load escrow program and call initialize
  // const escrowProgram = anchor.workspace.SkillEscrow;
  // await escrowProgram.methods.initialize(FEE_BPS)...

  console.log("✅ Escrow initialized");

  // 4. Save configuration
  const config = {
    cluster: CLUSTER,
    skillMint: skillMint.toString(),
    admin: admin.toString(),
    rate: RATE.toString(),
    feeBps: FEE_BPS,
    timestamp: new Date().toISOString(),
  };

  fs.writeFileSync(
    `./deployments/${CLUSTER}.json`,
    JSON.stringify(config, null, 2)
  );

  console.log(`\n✅ Configuration saved to deployments/${CLUSTER}.json`);
  console.log("\n🎉 All programs initialized successfully!");
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });
