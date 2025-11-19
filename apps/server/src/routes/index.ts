import { Express } from "express";
import { authRoutes } from "./auth";
import { matchRoutes } from "./match";
import { userRoutes } from "./user";

export function setupRoutes(app: Express) {
  app.use("/api/auth", authRoutes);
  app.use("/api/matches", matchRoutes);
  app.use("/api/users", userRoutes);
}
