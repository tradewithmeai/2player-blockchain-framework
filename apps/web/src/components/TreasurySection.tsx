"use client";

import { useState } from "react";
import { useWallet, useConnection } from "@solana/wallet-adapter-react";
import { PublicKey } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";

export function TreasurySection() {
  const { connection } = useConnection();
  const { publicKey, sendTransaction } = useWallet();
  const [solAmount, setSolAmount] = useState("");
  const [skillAmount, setSkillAmount] = useState("");
  const [loading, setLoading] = useState(false);

  const handleDeposit = async () => {
    if (!publicKey || !solAmount) return;

    setLoading(true);
    try {
      // TODO: Implement deposit using SDK
      // const tx = await buildDepositTransaction(...)
      // await sendTransaction(tx, connection)

      alert(`Depositing ${solAmount} SOL...`);
    } catch (error) {
      console.error("Deposit error:", error);
      alert("Deposit failed");
    } finally {
      setLoading(false);
    }
  };

  const handleRedeem = async () => {
    if (!publicKey || !skillAmount) return;

    setLoading(true);
    try {
      // TODO: Implement redeem using SDK
      // const tx = await buildRedeemTransaction(...)
      // await sendTransaction(tx, connection)

      alert(`Redeeming ${skillAmount} SKILL...`);
    } catch (error) {
      console.error("Redeem error:", error);
      alert("Redeem failed");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
      <div className="card">
        <h3 className="text-xl font-bold mb-4">Deposit SOL → SKILL</h3>
        <div className="space-y-4">
          <div>
            <label className="block text-sm text-gray-400 mb-2">
              SOL Amount
            </label>
            <input
              type="number"
              step="0.1"
              value={solAmount}
              onChange={(e) => setSolAmount(e.target.value)}
              className="w-full px-4 py-2 bg-gray-700 rounded-lg"
              placeholder="0.0"
            />
          </div>
          <button
            onClick={handleDeposit}
            disabled={loading || !solAmount}
            className="btn-primary w-full"
          >
            {loading ? "Processing..." : "Deposit"}
          </button>
        </div>
      </div>

      <div className="card">
        <h3 className="text-xl font-bold mb-4">Redeem SKILL → SOL</h3>
        <div className="space-y-4">
          <div>
            <label className="block text-sm text-gray-400 mb-2">
              SKILL Amount
            </label>
            <input
              type="number"
              step="1"
              value={skillAmount}
              onChange={(e) => setSkillAmount(e.target.value)}
              className="w-full px-4 py-2 bg-gray-700 rounded-lg"
              placeholder="0"
            />
          </div>
          <button
            onClick={handleRedeem}
            disabled={loading || !skillAmount}
            className="btn-primary w-full"
          >
            {loading ? "Processing..." : "Redeem"}
          </button>
        </div>
      </div>
    </div>
  );
}
