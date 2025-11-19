import { WebSocketServer, WebSocket } from "ws";
import { logger } from "../utils/logger";
import { prisma } from "../main";

interface Client {
  ws: WebSocket;
  userId?: string;
  matchId?: string;
}

const clients = new Map<string, Client>();

export function setupWebSocket(wss: WebSocketServer) {
  wss.on("connection", (ws: WebSocket) => {
    const clientId = generateClientId();
    clients.set(clientId, { ws });

    logger.info(`WebSocket client connected: ${clientId}`);

    ws.on("message", async (data: Buffer) => {
      try {
        const message = JSON.parse(data.toString());
        await handleMessage(clientId, message);
      } catch (error) {
        logger.error("WebSocket message error:", error);
        ws.send(JSON.stringify({ type: "error", error: "Invalid message" }));
      }
    });

    ws.on("close", () => {
      logger.info(`WebSocket client disconnected: ${clientId}`);
      clients.delete(clientId);
    });

    ws.send(JSON.stringify({ type: "connected", clientId }));
  });
}

async function handleMessage(clientId: string, message: any) {
  const client = clients.get(clientId);
  if (!client) return;

  const { type, payload } = message;

  switch (type) {
    case "auth":
      // Authenticate client
      client.userId = payload.userId;
      client.ws.send(JSON.stringify({ type: "auth_success" }));
      break;

    case "join_match":
      // Join a match room
      client.matchId = payload.matchId;
      broadcastToMatch(payload.matchId, {
        type: "player_joined",
        userId: client.userId,
      });
      break;

    case "leave_match":
      client.matchId = undefined;
      break;

    case "game_move":
      // Broadcast move to opponent
      if (client.matchId) {
        broadcastToMatch(
          client.matchId,
          {
            type: "game_move",
            position: payload.position,
            player: client.userId,
          },
          clientId
        );
      }
      break;

    case "game_state":
      // Broadcast game state update (for realtime games)
      if (client.matchId) {
        broadcastToMatch(
          client.matchId,
          {
            type: "game_state",
            state: payload.state,
          },
          clientId
        );
      }
      break;

    default:
      client.ws.send(JSON.stringify({ type: "error", error: "Unknown message type" }));
  }
}

function broadcastToMatch(matchId: string, message: any, excludeClientId?: string) {
  clients.forEach((client, id) => {
    if (client.matchId === matchId && id !== excludeClientId) {
      client.ws.send(JSON.stringify(message));
    }
  });
}

function generateClientId(): string {
  return Math.random().toString(36).substring(2, 15);
}

// Export for use in other modules
export function notifyMatch(matchId: string, message: any) {
  broadcastToMatch(matchId, message);
}
