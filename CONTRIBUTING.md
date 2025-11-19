# Contributing to Skill Gaming

Thank you for your interest in contributing! This guide will help you get started.

## Code of Conduct

Be respectful, inclusive, and collaborative.

## How to Contribute

### Reporting Bugs

1. Check if the bug has already been reported in [Issues](../../issues)
2. If not, create a new issue with:
   - Clear title and description
   - Steps to reproduce
   - Expected vs actual behavior
   - Your environment (OS, Solana version, etc.)
   - Relevant logs or screenshots

### Suggesting Features

1. Check [Issues](../../issues) for similar suggestions
2. Create a new issue with:
   - Clear description of the feature
   - Use cases and benefits
   - Possible implementation approach

### Pull Requests

1. **Fork** the repository
2. **Create a branch** from `develop`:
   ```bash
   git checkout -b feature/your-feature-name
   ```
3. **Make your changes**:
   - Follow the coding standards below
   - Add tests for new features
   - Update documentation
4. **Test your changes**:
   ```bash
   anchor test
   cd apps/server && yarn test
   cd apps/web && yarn build
   ```
5. **Commit** with clear messages:
   ```bash
   git commit -m "feat: add realtime chess game mode"
   ```
6. **Push** to your fork:
   ```bash
   git push origin feature/your-feature-name
   ```
7. **Open a Pull Request** to `develop` branch

## Development Setup

See [QUICKSTART.md](./QUICKSTART.md) for detailed setup instructions.

## Coding Standards

### Rust/Anchor Programs

- Use `rustfmt` for formatting
- Run `cargo clippy` and fix all warnings
- Add inline comments for complex logic
- Use descriptive variable names
- Follow Anchor best practices:
  - PDAs for all program-owned accounts
  - Checked arithmetic (`checked_mul`, etc.)
  - Proper account constraints
  - Events for important state changes

**Example**:
```rust
/// Transfer tokens to escrow vault
///
/// # Arguments
/// * `amount` - Amount to transfer (checked against stake)
pub fn fund(ctx: Context<Fund>) -> Result<()> {
    require!(
        ctx.accounts.player_ata.amount >= ctx.accounts.match_account.stake,
        EscrowError::InsufficientBalance
    );

    // ... implementation
}
```

### TypeScript/JavaScript

- Use TypeScript for all new code
- Follow ESLint rules
- Use meaningful variable names
- Add JSDoc comments for public APIs
- Prefer `async/await` over promises

**Example**:
```typescript
/**
 * Create a new match on-chain
 *
 * @param stake - Amount to wager in atomic SKILL units
 * @param mode - Game mode (turn-based or realtime)
 * @returns Transaction signature
 */
async function createMatch(stake: BN, mode: GameMode): Promise<string> {
  // ... implementation
}
```

### React/Next.js

- Use functional components with hooks
- Extract reusable logic to custom hooks
- Keep components small and focused
- Use TypeScript interfaces for props
- Follow Next.js conventions (app router)

**Example**:
```tsx
interface GameBoardProps {
  board: number[];
  onMove: (position: number) => void;
  disabled?: boolean;
}

export function GameBoard({ board, onMove, disabled = false }: GameBoardProps) {
  // ... implementation
}
```

## Testing

### Anchor Programs

```bash
# Run all program tests
anchor test

# Test specific program
anchor test --skip-build -- --test skill_treasury
```

Tests should:
- Cover happy path and error cases
- Use descriptive test names
- Clean up state between tests
- Test security constraints

### Backend

```bash
cd apps/server
yarn test
```

Tests should:
- Mock external dependencies
- Test API endpoints
- Test WebSocket events
- Cover authentication flows

### Frontend

```bash
cd apps/web
yarn test
```

Tests should:
- Test component rendering
- Test user interactions
- Mock wallet connections
- Test state management

## Project Structure

```
/
├── programs/           # Anchor programs
│   ├── skill_treasury/
│   ├── skill_escrow/
│   └── ttt_onchain/
├── tests/             # Anchor integration tests
├── packages/
│   └── sdk/           # TypeScript SDK
├── apps/
│   ├── server/        # Backend API
│   └── web/           # Frontend
├── scripts/           # Deployment and utility scripts
└── .github/
    └── workflows/     # CI/CD
```

## Git Workflow

1. `main` - Production-ready code
2. `develop` - Integration branch
3. Feature branches - Individual features

### Branch Naming

- `feature/add-chess-mode`
- `fix/escrow-timeout-bug`
- `docs/update-api-guide`
- `refactor/simplify-treasury`

### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

- `feat: add new game mode`
- `fix: correct timeout calculation`
- `docs: update deployment guide`
- `test: add escrow settlement tests`
- `refactor: simplify treasury logic`
- `chore: update dependencies`

## Security

### Reporting Vulnerabilities

**DO NOT** open public issues for security vulnerabilities.

Instead, email security@example.com with:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

### Security Best Practices

When contributing:
- Never commit private keys or secrets
- Validate all user inputs
- Use PDAs for program-owned accounts
- Check arithmetic for overflows
- Verify all account constraints
- Follow the [Solana Security Best Practices](https://docs.solana.com/developing/programming-model/security)

## Adding New Games

To add a new game:

1. **Create Anchor program** (if turn-based):
   ```bash
   anchor init my_game
   ```
2. **Integrate with escrow**:
   - Add CPI to `skill_escrow::settle_onchain`
   - Emit events for UI updates
3. **Add SDK methods** in `packages/sdk`
4. **Create UI components** in `apps/web`
5. **Add WebSocket handlers** (if realtime)
6. **Write tests**
7. **Update documentation**

## Documentation

When adding features:

- Update README.md if needed
- Add inline code comments
- Update API documentation
- Add examples to QUICKSTART.md

## Questions?

- Open a [Discussion](../../discussions)
- Join our Discord [link]
- Check existing [Issues](../../issues)

## Recognition

Contributors will be:
- Listed in CONTRIBUTORS.md
- Credited in release notes
- Mentioned on social media (with permission)

Thank you for contributing to Skill Gaming! 🎮⚡
