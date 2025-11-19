#!/bin/bash

# Deployment script for Skill Gaming programs

set -e

CLUSTER=${1:-devnet}

echo "🚀 Deploying to $CLUSTER"

# Build programs
echo "📦 Building programs..."
anchor build

# Deploy programs
echo "🌐 Deploying programs to $CLUSTER..."
anchor deploy --provider.cluster $CLUSTER

# Get program IDs
TREASURY_ID=$(solana-keygen pubkey target/deploy/skill_treasury-keypair.json)
ESCROW_ID=$(solana-keygen pubkey target/deploy/skill_escrow-keypair.json)
TTT_ID=$(solana-keygen pubkey target/deploy/ttt_onchain-keypair.json)

echo ""
echo "✅ Deployment complete!"
echo ""
echo "Program IDs:"
echo "  skill_treasury: $TREASURY_ID"
echo "  skill_escrow:   $ESCROW_ID"
echo "  ttt_onchain:    $TTT_ID"
echo ""
echo "⚠️  Update these IDs in:"
echo "  - Anchor.toml"
echo "  - apps/server/.env"
echo "  - apps/web/.env.local"
echo "  - packages/sdk/src/constants.ts"
echo ""
echo "🔧 Next steps:"
echo "  1. Update program IDs in config files"
echo "  2. Create and initialize SKILL mint"
echo "  3. Initialize treasury and escrow programs"
echo "  4. Deploy backend and frontend"
