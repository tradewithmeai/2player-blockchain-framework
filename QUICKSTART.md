# Quick Start Guide

Get up and running with Skill Gaming in 10 minutes.

## Prerequisites Check

```bash
# Verify installations
rustc --version      # Should be 1.75+
solana --version     # Should be 1.18.22
anchor --version     # Should be 0.30.1
node --version       # Should be 20+
```

## Step 1: Clone and Install (2 minutes)

```bash
git clone <repository-url>
cd 2player-blockchain-framework
yarn install
```

## Step 2: Build Programs (3 minutes)

```bash
anchor build
```

## Step 3: Start Local Validator (1 minute)

In a **new terminal**:

```bash
solana-test-validator
```

Keep this running!

## Step 4: Deploy to Localnet (2 minutes)

```bash
anchor deploy
```

Copy the program IDs shown and update:
- `Anchor.toml` (programs.localnet section)
- `packages/sdk/src/constants.ts`

## Step 5: Initialize Programs (1 minute)

```bash
# Create SKILL mint and initialize programs
ts-node scripts/init-programs.ts localnet
```

Save the SKILL mint address shown!

## Step 6: Setup Backend Database (2 minutes)

```bash
cd apps/server

# Copy and edit .env
cp .env.example .env

# For quick start, use SQLite (edit DATABASE_URL):
# DATABASE_URL="file:./dev.db"

# Or use Neon (recommended):
# Sign up at https://neon.tech
# Copy connection string to DATABASE_URL

# Run migrations
yarn prisma migrate dev

cd ../..
```

## Step 7: Start Backend (30 seconds)

In a **new terminal**:

```bash
cd apps/server
yarn dev
```

You should see:
```
🚀 Server running on port 3001
📡 WebSocket server ready
```

## Step 8: Start Frontend (30 seconds)

In a **new terminal**:

```bash
cd apps/web

# Copy and edit .env.local
cp .env.local.example .env.local

# Update with your SKILL mint and program IDs

yarn dev
```

## Step 9: Connect Wallet

1. Open [http://localhost:3000](http://localhost:3000)
2. Click "Connect Wallet"
3. Select Phantom (or install from [phantom.app](https://phantom.app))
4. Switch to **Devnet** in wallet settings
5. Get devnet SOL from [solfaucet.com](https://solfaucet.com)

## Step 10: Play!

1. **Deposit SOL** to mint SKILL tokens
2. **Create a match** in the lobby
3. **Join** from another browser/wallet
4. **Play Tic-Tac-Toe!**

## Troubleshooting

### "Transaction simulation failed"
- Check you're on the right cluster (localnet/devnet)
- Verify program IDs match in all config files
- Ensure you have enough SOL

### "Connection refused" errors
- Make sure validator is running (`solana-test-validator`)
- Check backend is running on port 3001
- Verify RPC endpoint in `.env` files

### Frontend won't load
- Run `yarn build` in packages/sdk first
- Clear Next.js cache: `rm -rf apps/web/.next`
- Check browser console for errors

## Next Steps

- Read the [README.md](./README.md) for full documentation
- Review [Architecture](#architecture) section
- Check out program code in `programs/`
- Customize the UI in `apps/web/`

## Key Concepts Recap

**SKILL Token**: Backed 1:1 by SOL, redeemable anytime

**Match Flow**:
1. Create match → 2. Join → 3. Fund (both players) → 4. Play → 5. Winner gets tokens

**Two Game Modes**:
- **Turn-based**: All logic on-chain (Tic-Tac-Toe)
- **Realtime**: Server-authoritative with dual signatures (future)

## Development Tips

```bash
# Watch Anchor builds
anchor build --watch

# Watch backend changes
cd apps/server && yarn dev

# Watch frontend changes
cd apps/web && yarn dev

# View program logs
solana logs

# View database
cd apps/server && yarn prisma studio
```

## Production Checklist

Before deploying to mainnet:

- [ ] Test thoroughly on devnet
- [ ] Audit smart contracts
- [ ] Set programs to immutable with `--final`
- [ ] Setup monitoring and alerts
- [ ] Configure production RPC endpoints
- [ ] Setup proper database backups
- [ ] Add rate limiting to API
- [ ] Enable CORS for production domain only
- [ ] Review legal requirements for your jurisdiction

---

**Need help?** Open an issue on GitHub!
