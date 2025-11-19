import { Router } from "express";
import { PublicKey } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
import { prisma } from "../main";
import { logger } from "../utils/logger";
import { generateMatchId } from "@skill-gaming/sdk";

const router = Router();

/**
 * Get all active matches (lobby)
 */
router.get("/", async (req, res) => {
  try {
    const matches = await prisma.match.findMany({
      where: {
        status: "CREATED",
        playerB: null,
      },
      include: {
        player1: true,
      },
      orderBy: {
        createdAt: "desc",
      },
    });

    res.json({ matches });
  } catch (error) {
    logger.error("Get matches error:", error);
    res.status(500).json({ error: "Internal server error" });
  }
});

/**
 * Get match by ID
 */
router.get("/:id", async (req, res) => {
  try {
    const { id } = req.params;

    const match = await prisma.match.findUnique({
      where: { id },
      include: {
        player1: true,
        player2: true,
        moves: {
          orderBy: { ply: "asc" },
        },
      },
    });

    if (!match) {
      return res.status(404).json({ error: "Match not found" });
    }

    res.json({ match });
  } catch (error) {
    logger.error("Get match error:", error);
    res.status(500).json({ error: "Internal server error" });
  }
});

/**
 * Create a new match
 */
router.post("/", async (req, res) => {
  try {
    const { playerA, stake, mode } = req.body;

    if (!playerA || !stake || !mode) {
      return res.status(400).json({ error: "Missing required fields" });
    }

    // Validate player exists
    const profile = await prisma.profile.findUnique({
      where: { walletPubkey: playerA },
    });

    if (!profile) {
      return res.status(404).json({ error: "Player profile not found" });
    }

    // Generate match ID
    const matchIdBuffer = generateMatchId();
    const matchId = matchIdBuffer.toString("hex");

    // Create match in DB
    const match = await prisma.match.create({
      data: {
        matchId,
        matchPda: "pending", // Will be updated after on-chain creation
        playerA,
        stake: BigInt(stake),
        mode,
        status: "CREATED",
      },
      include: {
        player1: true,
      },
    });

    logger.info(`Match created: ${match.id} by ${playerA}`);

    res.json({
      match,
      matchIdBuffer: matchId,
    });
  } catch (error) {
    logger.error("Create match error:", error);
    res.status(500).json({ error: "Internal server error" });
  }
});

/**
 * Join a match
 */
router.post("/:id/join", async (req, res) => {
  try {
    const { id } = req.params;
    const { playerB } = req.body;

    if (!playerB) {
      return res.status(400).json({ error: "playerB required" });
    }

    const match = await prisma.match.findUnique({
      where: { id },
    });

    if (!match) {
      return res.status(404).json({ error: "Match not found" });
    }

    if (match.playerB) {
      return res.status(409).json({ error: "Match already full" });
    }

    if (match.playerA === playerB) {
      return res.status(400).json({ error: "Cannot play against yourself" });
    }

    // Validate player exists
    const profile = await prisma.profile.findUnique({
      where: { walletPubkey: playerB },
    });

    if (!profile) {
      return res.status(404).json({ error: "Player profile not found" });
    }

    const updatedMatch = await prisma.match.update({
      where: { id },
      data: { playerB },
      include: {
        player1: true,
        player2: true,
      },
    });

    logger.info(`Player ${playerB} joined match ${id}`);

    res.json({ match: updatedMatch });
  } catch (error) {
    logger.error("Join match error:", error);
    res.status(500).json({ error: "Internal server error" });
  }
});

/**
 * Update match status (called by WebSocket/client after on-chain events)
 */
router.patch("/:id/status", async (req, res) => {
  try {
    const { id } = req.params;
    const { status, matchPda, winner, settledAt } = req.body;

    const updateData: any = {};

    if (status) updateData.status = status;
    if (matchPda) updateData.matchPda = matchPda;
    if (winner !== undefined) updateData.winner = winner;
    if (settledAt) updateData.settledAt = new Date(settledAt);

    const match = await prisma.match.update({
      where: { id },
      data: updateData,
    });

    res.json({ match });
  } catch (error) {
    logger.error("Update match status error:", error);
    res.status(500).json({ error: "Internal server error" });
  }
});

/**
 * Record a move (for turn-based games)
 */
router.post("/:id/moves", async (req, res) => {
  try {
    const { id } = req.params;
    const { position, actor } = req.body;

    const match = await prisma.match.findUnique({
      where: { id },
      include: { moves: true },
    });

    if (!match) {
      return res.status(404).json({ error: "Match not found" });
    }

    const ply = match.moves.length;

    const move = await prisma.move.create({
      data: {
        matchId: id,
        ply,
        position,
        actor,
      },
    });

    res.json({ move });
  } catch (error) {
    logger.error("Record move error:", error);
    res.status(500).json({ error: "Internal server error" });
  }
});

export { router as matchRoutes };
