import { Router } from "express";
import { sign } from "tweetnacl";
import bs58 from "bs58";
import { PublicKey } from "@solana/web3.js";
import { prisma } from "../main";
import { logger } from "../utils/logger";
import { v4 as uuidv4 } from "uuid";

const router = Router();

/**
 * SIWS (Sign-In With Solana) implementation
 */

// Step 1: Request a challenge
router.post("/challenge", async (req, res) => {
  try {
    const { walletPubkey } = req.body;

    if (!walletPubkey) {
      return res.status(400).json({ error: "walletPubkey required" });
    }

    // Validate pubkey
    try {
      new PublicKey(walletPubkey);
    } catch (err) {
      return res.status(400).json({ error: "Invalid wallet pubkey" });
    }

    // Generate nonce
    const nonce = uuidv4();
    const expiresAt = new Date(Date.now() + 5 * 60 * 1000); // 5 minutes

    // Save challenge
    await prisma.siwsChallenge.create({
      data: {
        walletPubkey,
        nonce,
        expiresAt,
      },
    });

    const message = `Sign this message to authenticate with Skill Gaming.\n\nNonce: ${nonce}`;

    res.json({ message, nonce });
  } catch (error) {
    logger.error("Challenge error:", error);
    res.status(500).json({ error: "Internal server error" });
  }
});

// Step 2: Verify signature and register/login
router.post("/verify", async (req, res) => {
  try {
    const { walletPubkey, signature, nonce, username } = req.body;

    if (!walletPubkey || !signature || !nonce) {
      return res.status(400).json({ error: "Missing required fields" });
    }

    // Find challenge
    const challenge = await prisma.siwsChallenge.findFirst({
      where: {
        walletPubkey,
        nonce,
        expiresAt: { gte: new Date() },
      },
    });

    if (!challenge) {
      return res.status(401).json({ error: "Invalid or expired challenge" });
    }

    // Verify signature
    const message = `Sign this message to authenticate with Skill Gaming.\n\nNonce: ${nonce}`;
    const messageBytes = new TextEncoder().encode(message);
    const signatureBytes = bs58.decode(signature);
    const pubkeyBytes = new PublicKey(walletPubkey).toBytes();

    const isValid = sign.detached.verify(messageBytes, signatureBytes, pubkeyBytes);

    if (!isValid) {
      return res.status(401).json({ error: "Invalid signature" });
    }

    // Delete used challenge
    await prisma.siwsChallenge.delete({
      where: { id: challenge.id },
    });

    // Check if profile exists
    let profile = await prisma.profile.findUnique({
      where: { walletPubkey },
    });

    if (!profile) {
      // Register new user
      if (!username) {
        return res.status(400).json({ error: "Username required for new users" });
      }

      // Check username uniqueness
      const existingUsername = await prisma.profile.findUnique({
        where: { username },
      });

      if (existingUsername) {
        return res.status(409).json({ error: "Username already taken" });
      }

      profile = await prisma.profile.create({
        data: {
          walletPubkey,
          username,
        },
      });

      logger.info(`New user registered: ${username} (${walletPubkey})`);
    }

    res.json({
      success: true,
      profile: {
        id: profile.id,
        walletPubkey: profile.walletPubkey,
        username: profile.username,
      },
    });
  } catch (error) {
    logger.error("Verify error:", error);
    res.status(500).json({ error: "Internal server error" });
  }
});

export { router as authRoutes };
