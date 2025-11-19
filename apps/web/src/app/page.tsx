"use client";

import { useWallet } from "@solana/wallet-adapter-react";
import { WalletMultiButton } from "@solana/wallet-adapter-react-ui";
import Link from "next/link";
import { TreasurySection } from "@/components/TreasurySection";

export default function Home() {
  const { connected, publicKey } = useWallet();

  return (
    <main className="min-h-screen bg-gradient-to-b from-gray-900 to-black text-white">
      <nav className="p-6 flex justify-between items-center border-b border-gray-800">
        <h1 className="text-2xl font-bold">Skill Gaming</h1>
        <WalletMultiButton />
      </nav>

      <div className="container mx-auto px-6 py-12">
        {!connected ? (
          <div className="text-center py-20">
            <h2 className="text-4xl font-bold mb-4">
              Welcome to Skill Gaming
            </h2>
            <p className="text-xl text-gray-400 mb-8">
              Play 2-player games and wager SKILL tokens on Solana
            </p>
            <p className="text-lg text-gray-500">
              Connect your wallet to get started
            </p>
          </div>
        ) : (
          <div className="space-y-8">
            <div className="text-center">
              <h2 className="text-3xl font-bold mb-2">
                Welcome, {publicKey?.toString().slice(0, 8)}...
              </h2>
              <p className="text-gray-400">
                Deposit SOL to mint SKILL tokens and start playing
              </p>
            </div>

            <TreasurySection />

            <div className="grid grid-cols-1 md:grid-cols-2 gap-6 mt-12">
              <Link href="/lobby" className="card hover:shadow-xl transition">
                <h3 className="text-2xl font-bold mb-2">Play Tic-Tac-Toe</h3>
                <p className="text-gray-400">
                  Join a match or create your own
                </p>
              </Link>

              <Link
                href="/profile"
                className="card hover:shadow-xl transition"
              >
                <h3 className="text-2xl font-bold mb-2">Your Profile</h3>
                <p className="text-gray-400">
                  Set username and view match history
                </p>
              </Link>
            </div>
          </div>
        )}
      </div>
    </main>
  );
}
