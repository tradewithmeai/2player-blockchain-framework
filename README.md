# Skill Gaming - 2-Player Blockchain Gaming Framework

A complete framework for building skill-based 2-player games on Solana with wagering via SKILL tokens.

## Overview

This repository contains:

- **3 Anchor Programs** (Rust):
  - `skill_treasury`: SOL ↔ SKILL token mint/redeem gateway
  - `skill_escrow`: Match escrow and settlement with timeout/dispute logic
  - `ttt_onchain`: Trustless on-chain Tic-Tac-Toe game logic

- **Backend Server** (Node.js/TypeScript):
  - REST API for matchmaking and user management
  - WebSocket server for real-time gameplay
  - SIWS (Sign-In With Solana) authentication
  - Result attestation service for realtime games
  - Prisma + PostgreSQL database

- **Frontend** (Next.js/React):
  - Solana wallet integration (Phantom, Solflare, Coinbase, Backpack)
  - Deposit/redeem SKILL tokens
  - Game lobby and matchmaking
  - Tic-Tac-Toe gameplay UI

- **TypeScript SDK**:
  - Client libraries for all programs
  - Helper functions for transactions
  - Ed25519 signature utilities

## Architecture

### On-Chain Programs

1. **skill_treasury**
   - Fixed-rate SOL ↔ SKILL conversion
   - Fully backed by SOL in treasury vault
   - Admin-configurable rate and fees

2. **skill_escrow**
   - Creates and manages matches
   - Escrows SKILL tokens from both players
   - Settles to winner via:
     - CPI from game programs (turn-based)
     - Ed25519 dual signatures (realtime)
   - Timeout and cancellation logic

3. **ttt_onchain**
   - Validates all Tic-Tac-Toe moves on-chain
   - Detects wins and draws
   - Handles timeouts
   - CPIs to escrow for settlement

### Game Modes

**Turn-Based (Tic-Tac-Toe)**
- All game state and logic on-chain
- Trustless settlement via CPI
- Timeout protection

**Realtime (Future)**
- Server-authoritative state
- Both players sign final result off-chain
- Settlement requires ed25519 precompile verification

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (latest stable)
- [Solana CLI](https://docs.solana.com/cli/install-solana-cli-tools) v1.18.22
- [Anchor](https://www.anchor-lang.com/docs/installation) v0.30.1
- [Node.js](https://nodejs.org/) v20+
- [Yarn](https://yarnpkg.com/)
- PostgreSQL (or Neon for serverless)

### Installation

```bash
# Clone the repository
git clone https://github.com/your-org/2player-blockchain-framework.git
cd 2player-blockchain-framework

# Install dependencies
yarn install

# Build programs
anchor build

# Generate Prisma client
cd apps/server
yarn prisma generate
```

### Local Development

#### 1. Start Local Validator

```bash
solana-test-validator
```

#### 2. Deploy Programs

```bash
anchor deploy
```

#### 3. Initialize Programs

```bash
# Create SKILL mint and initialize programs
ts-node scripts/init-programs.ts localnet
```

#### 4. Setup Database

```bash
cd apps/server

# Copy environment variables
cp .env.example .env

# Update DATABASE_URL in .env

# Run migrations
yarn prisma migrate dev

# (Optional) Open Prisma Studio
yarn prisma studio
```

#### 5. Start Backend

```bash
cd apps/server
yarn dev
```

#### 6. Start Frontend

```bash
cd apps/web
cp .env.local.example .env.local
# Update environment variables
yarn dev
```

Visit [http://localhost:3000](http://localhost:3000)

## Deployment

### Devnet

```bash
# Deploy programs
./scripts/deploy.sh devnet

# Update program IDs in config files

# Initialize programs
ts-node scripts/init-programs.ts devnet

# Deploy backend (Railway/Render)
cd apps/server
# Follow your hosting provider's deployment guide

# Deploy frontend (Vercel)
cd apps/web
vercel deploy
```

### Mainnet

⚠️ **WARNING**: Programs are **immutable** on mainnet. Thoroughly test on devnet first.

```bash
# Deploy programs
./scripts/deploy.sh mainnet

# Set programs to immutable (IRREVERSIBLE)
solana program set-upgrade-authority <PROGRAM_ID> --final

# Initialize programs
ts-node scripts/init-programs.ts mainnet
```

## Testing

```bash
# Test Anchor programs
anchor test

# Test backend
cd apps/server
yarn test

# Test frontend
cd apps/web
yarn test
```

## Repository Structure

```
/
├── programs/              # Anchor programs (Rust)
│   ├── skill_treasury/
│   ├── skill_escrow/
│   └── ttt_onchain/
├── apps/
│   ├── server/           # Backend API + WebSocket
│   └── web/              # Next.js frontend
├── packages/
│   └── sdk/              # TypeScript SDK
├── tests/                # Anchor tests
├── scripts/              # Deployment scripts
└── .github/workflows/    # CI/CD
```

## Key Concepts

### Treasury (SOL ↔ SKILL)

- Users deposit SOL to mint SKILL tokens
- SKILL is backed 1:1 by SOL in the treasury vault
- Users can redeem SKILL for SOL at any time
- Fixed exchange rate (configurable by admin)

### Match Flow

1. **Create**: Player A creates a match with a stake
2. **Join**: Player B joins the match
3. **Fund**: Both players transfer SKILL to escrow
4. **Start**: Match becomes active when both funded
5. **Play**: Players make moves (on-chain or via WebSocket)
6. **Settle**: Winner receives `(2 × stake) - fee`

### Security Features

- ✅ Account validation (PDAs, constraints)
- ✅ Checked arithmetic (no overflow)
- ✅ Token program ID enforcement (classic SPL only)
- ✅ Ed25519 signature verification for realtime games
- ✅ Timeout protection
- ✅ Immutable programs on mainnet

## API Documentation

### REST Endpoints

```
POST   /api/auth/challenge       - Request SIWS challenge
POST   /api/auth/verify          - Verify signature and login
GET    /api/matches              - List open matches
POST   /api/matches              - Create match
GET    /api/matches/:id          - Get match details
POST   /api/matches/:id/join     - Join match
PATCH  /api/matches/:id/status   - Update match status
POST   /api/matches/:id/moves    - Record move
GET    /api/users/:wallet        - Get user profile
GET    /api/users/:wallet/matches - Get match history
```

### WebSocket Events

```javascript
// Client -> Server
{ type: "auth", payload: { userId } }
{ type: "join_match", payload: { matchId } }
{ type: "game_move", payload: { position } }

// Server -> Client
{ type: "player_joined", userId }
{ type: "game_move", position, player }
{ type: "game_state", state }
```

## Configuration

### Environment Variables

**Backend** (`apps/server/.env`):
- `DATABASE_URL` - Neon Postgres connection string
- `RPC_ENDPOINT` - Solana RPC endpoint
- `SKILL_MINT` - SKILL token mint address
- Program IDs

**Frontend** (`apps/web/.env.local`):
- `NEXT_PUBLIC_RPC_ENDPOINT` - Solana RPC endpoint
- `NEXT_PUBLIC_API_URL` - Backend API URL
- `NEXT_PUBLIC_WS_URL` - WebSocket URL
- `NEXT_PUBLIC_SKILL_MINT` - SKILL token mint

## Troubleshooting

### Programs won't deploy
- Ensure Solana CLI version matches `Anchor.toml`
- Check you have enough SOL for deployment
- Verify `declare_id!` matches program ID

### Transactions failing
- Add compute budget instructions
- Check account constraints
- Verify token mint matches everywhere

### WebSocket not connecting
- Ensure backend is running on correct port
- Check CORS settings
- Verify WSS endpoint if using HTTPS

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests
5. Submit a pull request

## License

MIT

## Resources

- [Solana Documentation](https://docs.solana.com/)
- [Anchor Book](https://book.anchor-lang.com/)
- [SPL Token Docs](https://spl.solana.com/token)
- [Solana Cookbook](https://solanacookbook.com/)

## Support

For issues and questions:
- GitHub Issues
- Discord: [Your Discord]
- Twitter: [@YourTwitter]

---

Built with ⚡ on Solana
