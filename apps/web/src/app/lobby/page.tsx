"use client";

import { useEffect, useState } from "react";
import { useWallet } from "@solana/wallet-adapter-react";
import { WalletMultiButton } from "@solana/wallet-adapter-react-ui";
import Link from "next/link";

interface Match {
  id: string;
  matchId: string;
  stake: string;
  mode: string;
  player1: {
    username: string;
    walletPubkey: string;
  };
}

export default function LobbyPage() {
  const { connected } = useWallet();
  const [matches, setMatches] = useState<Match[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    fetchMatches();
  }, []);

  const fetchMatches = async () => {
    try {
      const response = await fetch(
        `${process.env.NEXT_PUBLIC_API_URL}/api/matches`
      );
      const data = await response.json();
      setMatches(data.matches);
    } catch (error) {
      console.error("Failed to fetch matches:", error);
    } finally {
      setLoading(false);
    }
  };

  return (
    <main className="min-h-screen bg-gradient-to-b from-gray-900 to-black text-white">
      <nav className="p-6 flex justify-between items-center border-b border-gray-800">
        <Link href="/" className="text-2xl font-bold">
          Skill Gaming
        </Link>
        <WalletMultiButton />
      </nav>

      <div className="container mx-auto px-6 py-12">
        <div className="flex justify-between items-center mb-8">
          <h1 className="text-3xl font-bold">Game Lobby</h1>
          <Link href="/lobby/create" className="btn-primary">
            Create Match
          </Link>
        </div>

        {loading ? (
          <p className="text-center text-gray-400">Loading matches...</p>
        ) : matches.length === 0 ? (
          <p className="text-center text-gray-400">
            No open matches. Create one to get started!
          </p>
        ) : (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
            {matches.map((match) => (
              <div key={match.id} className="card hover:shadow-xl transition">
                <div className="flex justify-between items-start mb-4">
                  <div>
                    <h3 className="font-bold text-lg">Tic-Tac-Toe</h3>
                    <p className="text-sm text-gray-400">
                      {match.mode === "turn_based" ? "Turn-based" : "Realtime"}
                    </p>
                  </div>
                  <div className="text-right">
                    <p className="text-xl font-bold text-secondary">
                      {(parseInt(match.stake) / 1_000_000).toFixed(2)} SKILL
                    </p>
                  </div>
                </div>
                <p className="text-sm text-gray-400 mb-4">
                  Created by: {match.player1.username}
                </p>
                <Link
                  href={`/match/${match.id}`}
                  className="btn-primary w-full block text-center"
                >
                  Join Match
                </Link>
              </div>
            ))}
          </div>
        )}
      </div>
    </main>
  );
}
