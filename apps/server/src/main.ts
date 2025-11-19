import express from "express";
import cors from "cors";
import { createServer } from "http";
import { WebSocketServer } from "ws";
import dotenv from "dotenv";
import { PrismaClient } from "@prisma/client";
import { Connection, PublicKey } from "@solana/web3.js";

import { setupRoutes } from "./routes";
import { setupWebSocket } from "./websocket";
import { logger } from "./utils/logger";

dotenv.config();

const app = express();
const server = createServer(app);
const wss = new WebSocketServer({ server });

// Initialize Prisma
export const prisma = new PrismaClient();

// Initialize Solana connection
export const solanaConnection = new Connection(
  process.env.RPC_ENDPOINT || "https://api.devnet.solana.com",
  {
    commitment: "confirmed",
    wsEndpoint: process.env.WSS_ENDPOINT,
  }
);

// Middleware
app.use(cors({ origin: process.env.CORS_ORIGIN || "http://localhost:3000" }));
app.use(express.json());

// Health check
app.get("/health", (req, res) => {
  res.json({ status: "ok", timestamp: new Date().toISOString() });
});

// Setup routes
setupRoutes(app);

// Setup WebSocket
setupWebSocket(wss);

const PORT = process.env.PORT || 3001;

server.listen(PORT, () => {
  logger.info(`🚀 Server running on port ${PORT}`);
  logger.info(`📡 WebSocket server ready`);
  logger.info(`🔗 Solana cluster: ${process.env.CLUSTER || "devnet"}`);
});

// Graceful shutdown
process.on("SIGTERM", async () => {
  logger.info("SIGTERM received, shutting down gracefully");
  await prisma.$disconnect();
  server.close(() => {
    logger.info("Server closed");
    process.exit(0);
  });
});
