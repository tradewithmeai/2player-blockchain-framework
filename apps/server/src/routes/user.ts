import { Router } from "express";
import { prisma } from "../main";
import { logger } from "../utils/logger";

const router = Router();

/**
 * Get user profile by wallet pubkey
 */
router.get("/:walletPubkey", async (req, res) => {
  try {
    const { walletPubkey } = req.params;

    const profile = await prisma.profile.findUnique({
      where: { walletPubkey },
    });

    if (!profile) {
      return res.status(404).json({ error: "Profile not found" });
    }

    res.json({ profile });
  } catch (error) {
    logger.error("Get user error:", error);
    res.status(500).json({ error: "Internal server error" });
  }
});

/**
 * Check username availability
 */
router.get("/check/:username", async (req, res) => {
  try {
    const { username } = req.params;

    const existing = await prisma.profile.findUnique({
      where: { username },
    });

    res.json({ available: !existing });
  } catch (error) {
    logger.error("Check username error:", error);
    res.status(500).json({ error: "Internal server error" });
  }
});

/**
 * Get user match history
 */
router.get("/:walletPubkey/matches", async (req, res) => {
  try {
    const { walletPubkey } = req.params;

    const matches = await prisma.match.findMany({
      where: {
        OR: [{ playerA: walletPubkey }, { playerB: walletPubkey }],
      },
      include: {
        player1: true,
        player2: true,
      },
      orderBy: {
        createdAt: "desc",
      },
    });

    res.json({ matches });
  } catch (error) {
    logger.error("Get match history error:", error);
    res.status(500).json({ error: "Internal server error" });
  }
});

export { router as userRoutes };
