# Documentation Guide

Complete guide for generating, viewing, and maintaining documentation for Skill Gaming programs.

## Quick Start

Generate all documentation:

```bash
./scripts/generate-docs.sh --open
```

This will:
1. Build all programs
2. Generate rustdoc for each program
3. Extract error codes
4. Create index page
5. Open in your browser

## Documentation Types

### 1. Rustdoc (API Reference)

**What:** Auto-generated API documentation from source code comments.

**Location:** `target/doc/`

**Generate:**
```bash
./scripts/generate-docs.sh
```

**View:**
```bash
# Open in browser
open target/doc/index.html

# Or serve locally
cd target/doc && python3 -m http.server 8000
# Visit http://localhost:8000
```

**Programs:**
- [skill_treasury](../target/doc/skill_treasury/index.html) - Treasury operations
- [skill_escrow](../target/doc/skill_escrow/index.html) - Match escrow
- [ttt_onchain](../target/doc/ttt_onchain/index.html) - Tic-Tac-Toe game

### 2. Error Code Registry

**What:** Complete reference of all error codes with causes and resolutions.

**Location:** `docs/ERROR_CODES.md`

**Generate:**
```bash
./scripts/extract-errors.sh
```

**Output:**
- `docs/ERROR_CODES.md` - Manual reference (maintained by developers)
- `docs/ERROR_CODES_GENERATED.md` - Auto-generated quick reference

**Example:**
```markdown
### Error 6004: `InsufficientLiquidity`

**Message:** "Insufficient liquidity in treasury vault..."
**Cause:** Treasury vault doesn't have enough SOL...
**Resolution:**
1. Check treasury balance
2. Reduce redemption amount
```

### 3. Compute Benchmarks

**What:** Performance documentation showing compute unit usage.

**Location:** `target/benchmark/`

**Generate:**
```bash
./scripts/benchmark-compute.sh
```

**Output:**
- `target/benchmark/compute-units.md` - Comprehensive benchmark report
- `target/benchmark/compute-data.json` - Historical data (from tests)
- `target/benchmark/compute-report.md` - Auto-generated from test data

**See:** [BENCHMARKING.md](./BENCHMARKING.md) for full guide.

### 4. User Guides

**What:** Markdown documentation explaining concepts and workflows.

**Location:** `docs/`

**Files:**
- `README.md` - Project overview and setup
- `QUICKSTART.md` - Getting started guide
- `CONTRIBUTING.md` - Contribution guidelines
- `ERROR_CODES.md` - Error reference
- `BENCHMARKING.md` - Performance guide
- `DOCUMENTATION.md` - This file

## Writing Documentation

### Rustdoc Comments

Use triple-slash (`///`) for public items and `//!` for modules:

```rust
//! Module-level documentation
//!
//! # Overview
//! This module handles...

/// Function documentation
///
/// # Arguments
/// * `amount` - Amount to deposit
///
/// # Errors
/// Returns `InsufficientFunds` if...
///
/// # Example
/// ```
/// deposit(1000)?;
/// ```
pub fn deposit(amount: u64) -> Result<()> {
    // ...
}
```

### Documentation Sections

For instructions, include these sections:

1. **Arguments** - Parameter descriptions
2. **Security** - Security considerations
3. **Errors** - Possible error conditions
4. **Events** - Emitted events
5. **Example** - Usage example in TypeScript

**Template:**
```rust
/// Brief description of what instruction does.
///
/// Detailed explanation of behavior and use cases.
///
/// # Arguments
///
/// * `param1` - What this parameter does
/// * `param2` - What this parameter does
///
/// # Security
///
/// - Security consideration 1
/// - Security consideration 2
///
/// # Errors
///
/// - `ErrorName1`: When it occurs
/// - `ErrorName2`: When it occurs
///
/// # Events
///
/// - `EventName`: What it contains
///
/// # Example
///
/// ```typescript
/// await program.methods
///   .instructionName(param1, param2)
///   .accounts({ ... })
///   .rpc();
/// ```
pub fn instruction_name(ctx: Context<...>, param1: u64, param2: u8) -> Result<()> {
    // ...
}
```

### Error Documentation

Add to `docs/ERROR_CODES.md` when adding new errors:

```markdown
### Error 6XXX: `ErrorName`

**Message:** "Error message shown to user"

**Cause:**
Description of what causes this error.

**Resolution:**
1. Step 1 to resolve
2. Step 2 to resolve

**When it occurs:**
- `instruction_name` when condition

**Example:**
\`\`\`
Error code: 6XXX
Details: ...
\`\`\`
```

## Rustdoc Configuration

### Workspace Level

In root `Cargo.toml`:

```toml
[workspace.package]
version = "0.1.0"
authors = ["Skill Gaming Contributors"]
edition = "2021"
license = "MIT"
repository = "https://github.com/..."
documentation = "https://docs.rs/..."

[workspace.metadata.docs.rs]
all-features = true
rustdoc-args = ["--cfg", "docsrs"]
```

### Program Level

In each `programs/*/Cargo.toml`:

```toml
[package]
name = "skill_treasury"
description = "SOL to SKILL token mint/redeem treasury"
keywords = ["solana", "blockchain", "gaming"]
categories = ["cryptography::cryptocurrencies", "game-development"]

[package.metadata.docs.rs]
all-features = true
rustdoc-args = ["--cfg", "docsrs"]
```

## Hosting Documentation

### Option 1: GitHub Pages

```bash
# Generate docs
./scripts/generate-docs.sh

# Create gh-pages branch
git checkout --orphan gh-pages
git rm -rf .
cp -r target/doc/* .
git add .
git commit -m "Deploy documentation"
git push origin gh-pages

# Enable in GitHub repo settings
# Settings → Pages → Source: gh-pages branch
```

Access at: `https://yourusername.github.io/repo-name/`

### Option 2: docs.rs

For published crates on crates.io:

1. Publish to crates.io:
   ```bash
   cargo publish -p skill_treasury
   ```

2. docs.rs automatically builds docs

3. Access at: `https://docs.rs/skill_treasury`

### Option 3: Local HTTP Server

```bash
cd target/doc
python3 -m http.server 8000
```

Visit: `http://localhost:8000`

### Option 4: Netlify/Vercel

1. Generate docs:
   ```bash
   ./scripts/generate-docs.sh
   ```

2. Deploy `target/doc/` directory

3. Access at your custom domain

## CI/CD Integration

### GitHub Actions

Add to `.github/workflows/docs.yml`:

```yaml
name: Documentation

on:
  push:
    branches: [main, develop]
  pull_request:

jobs:
  docs:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Install Solana
        run: |
          sh -c "$(curl -sSfL https://release.solana.com/v1.18.22/install)"
          echo "$HOME/.local/share/solana/install/active_release/bin" >> $GITHUB_PATH

      - name: Install Anchor
        run: |
          cargo install --git https://github.com/coral-xyz/anchor --tag v0.30.1 anchor-cli --locked

      - name: Generate Documentation
        run: ./scripts/generate-docs.sh

      - name: Extract Error Codes
        run: ./scripts/extract-errors.sh

      - name: Upload Docs
        uses: actions/upload-artifact@v3
        with:
          name: documentation
          path: target/doc/

      - name: Deploy to GitHub Pages
        if: github.ref == 'refs/heads/main'
        uses: peaceiris/actions-gh-pages@v3
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}
          publish_dir: ./target/doc
```

## Documentation Checklist

When adding new features:

- [ ] Add rustdoc comments to all public items
- [ ] Include Arguments, Security, Errors, Events, Example sections
- [ ] Update ERROR_CODES.md for new error codes
- [ ] Run `./scripts/generate-docs.sh` to verify
- [ ] Run `./scripts/extract-errors.sh` to update auto-generated docs
- [ ] Update CHANGELOG.md with new features
- [ ] Update README.md if user-facing changes

## Documentation Standards

### Module-level Docs

Every program's `lib.rs` should have:

```rust
//! # Program Name
//!
//! Brief description.
//!
//! ## Overview
//!
//! Detailed explanation.
//!
//! ## Security Model
//!
//! Security considerations.
//!
//! ## Example Usage
//!
//! ```typescript
//! // Example code
//! ```
```

Minimum 30-50 lines of module docs for complex programs.

### Function Docs

Every public function needs:
- Brief description (one line)
- Detailed explanation (if needed)
- Arguments section
- At least one of: Security, Errors, Events, Example

### Type Docs

Document all public types:

```rust
/// Game state account
///
/// Stores complete state of a game including board position,
/// turn tracking, timeout deadline, and winner.
#[account]
pub struct Game {
    /// Match ID from escrow program (used as seed)
    pub match_id: [u8; 32],
    // ...
}
```

### Error Docs

In code:

```rust
/// 6004 - Insufficient liquidity in vault
#[msg("Insufficient liquidity in treasury vault...")]
InsufficientLiquidity,
```

In ERROR_CODES.md:

Complete entry with cause, resolution, examples.

## Maintenance

### Regular Tasks

**Weekly:**
- Review and update ERROR_CODES.md for new errors
- Run `./scripts/generate-docs.sh` after code changes

**Before Release:**
- Generate full documentation
- Review for accuracy
- Update version numbers
- Deploy to hosting

**After Release:**
- Tag documentation with version
- Archive old versions
- Update changelog

### Tools

**Check for missing docs:**
```bash
RUSTDOCFLAGS="-W missing_docs" cargo doc
```

**Check for broken links:**
```bash
cargo doc --no-deps
# Check warnings for broken intra-doc links
```

**Documentation coverage:**
```bash
# Install cargo-doc-coverage
cargo install cargo-doc-coverage

# Check coverage
cargo doc-coverage
```

## Troubleshooting

### "error: missing docs" warnings

Add documentation to flagged items or allow with:

```rust
#[allow(missing_docs)]
pub fn internal_function() { }
```

### Broken intra-doc links

Fix links to use proper format:

```rust
/// See [`OtherStruct`] for details
/// See [`other_function`] for usage
/// See [`crate::module::Item`] for cross-module
```

### Documentation not generating

1. Check Cargo.toml has metadata
2. Ensure all dependencies build
3. Run with verbose: `cargo doc -v`
4. Check for syntax errors in doc comments

### Missing private items

Add flag:
```bash
cargo doc --document-private-items
```

Or in script, already included.

## Resources

- [Rustdoc Guide](https://doc.rust-lang.org/rustdoc/)
- [Documentation Guidelines](https://rust-lang.github.io/api-guidelines/documentation.html)
- [Anchor Documentation](https://www.anchor-lang.com/docs)
- [Solana Documentation](https://docs.solana.com)

## Examples

See our programs for examples:
- `programs/skill_treasury/src/lib.rs` - Module docs
- `programs/skill_escrow/src/lib.rs` - Comprehensive instruction docs
- `programs/ttt_onchain/src/lib.rs` - Game logic documentation
- `docs/ERROR_CODES.md` - Error documentation

---

**Next Steps:**

1. Generate docs: `./scripts/generate-docs.sh --open`
2. Review generated documentation
3. Set up GitHub Pages or other hosting
4. Add documentation CI/CD workflow
5. Keep docs updated with code changes
