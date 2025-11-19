# Compute Unit Benchmarking Guide

This guide explains how to measure and optimize compute unit (CU) usage in Skill Gaming programs.

## Why Compute Units Matter

Solana has a **200,000 CU limit per transaction**. Exceeding this causes transaction failures.

**Targets:**
- ✅ **Good:** < 50,000 CU (25% of limit)
- ⚠️ **Warning:** 50,000 - 100,000 CU (25-50% of limit)
- ❌ **Critical:** > 100,000 CU (50%+ of limit)

## Quick Start

### 1. Run Benchmark Script

```bash
./scripts/benchmark-compute.sh
```

This generates a report at `target/benchmark/compute-units.md` with estimates for all instructions.

### 2. Profile a Specific Transaction

After running a transaction on devnet:

```bash
./scripts/profile-transaction.sh <signature> devnet
```

Example:
```bash
./scripts/profile-transaction.sh 5KZr3vJt8h9...abc123 devnet
```

### 3. Add Profiling to Tests

In your test file:

```typescript
import {
  measureComputeUnits,
  logComputeUnits,
  saveBenchmark,
  assertComputeLimit,
} from "./utils/compute-profiler";

it("should use reasonable compute units", async () => {
  // Execute instruction
  const sig = await program.methods
    .depositSolAndMintSkill(new BN(1_000_000))
    .accounts({
      config: configPda,
      treasuryVault: vaultPda,
      // ... other accounts
    })
    .rpc();

  // Measure compute units
  const result = await measureComputeUnits(provider.connection, sig);

  // Log to console
  logComputeUnits("deposit_sol_and_mint_skill", result);

  // Save to benchmark file
  saveBenchmark("skill_treasury", "deposit_sol_and_mint_skill", result);

  // Assert limits (fail test if > 50,000 CU)
  assertComputeLimit("deposit_sol_and_mint_skill", result, 50000);
});
```

## Tools Overview

### 1. `benchmark-compute.sh`

**Purpose:** Generate comprehensive benchmark report for all programs.

**Usage:**
```bash
./scripts/benchmark-compute.sh
```

**Output:** `target/benchmark/compute-units.md`

**Features:**
- Estimates for all instructions
- Status indicators (✅⚠️❌)
- Optimization recommendations
- CI/CD integration guide

### 2. `profile-transaction.sh`

**Purpose:** Analyze a specific transaction's compute usage.

**Usage:**
```bash
./scripts/profile-transaction.sh <signature> [network]
```

**Example:**
```bash
# Profile on devnet
./scripts/profile-transaction.sh 5KZr...abc devnet

# Profile on mainnet
./scripts/profile-transaction.sh 3Hx...xyz mainnet-beta
```

**Output:**
- Compute units consumed
- Percentage of limit used
- Status (good/warning/critical)
- Program logs
- Any errors
- Explorer link

### 3. `compute-profiler.ts`

**Purpose:** TypeScript utilities for test-based profiling.

**Key Functions:**

#### `measureComputeUnits(connection, signature)`
Returns compute unit usage for a transaction.

```typescript
const result = await measureComputeUnits(provider.connection, sig);
// result = { consumed: 25000, limit: 200000, percentage: 12.5, status: "good" }
```

#### `logComputeUnits(instructionName, result)`
Logs formatted output to console.

```typescript
logComputeUnits("deposit_sol_and_mint_skill", result);
// Output: ✅ deposit_sol_and_mint_skill: 25,000 CU (12.5%)
```

#### `saveBenchmark(program, instruction, result)`
Saves benchmark data to JSON file for historical tracking.

```typescript
saveBenchmark("skill_treasury", "deposit_sol_and_mint_skill", result);
// Appends to: target/benchmark/compute-data.json
```

#### `assertComputeLimit(instructionName, result, maxAllowed)`
Asserts compute usage is within limit (throws if exceeded).

```typescript
assertComputeLimit("deposit_sol_and_mint_skill", result, 50000);
// Throws if result.consumed > 50000
```

#### `generateReport(outputPath)`
Generates markdown report from saved benchmarks.

```typescript
generateReport(); // Creates target/benchmark/compute-report.md
```

## Benchmarking Workflow

### For Development

1. **Write tests with profiling:**
   ```typescript
   const sig = await program.methods.myInstruction().rpc();
   const result = await measureComputeUnits(connection, sig);
   logComputeUnits("myInstruction", result);
   saveBenchmark("my_program", "myInstruction", result);
   ```

2. **Run tests:**
   ```bash
   anchor test
   ```

3. **Generate report:**
   ```typescript
   // Add to test file
   after(async () => {
     generateReport();
   });
   ```

4. **Review:**
   ```bash
   cat target/benchmark/compute-report.md
   ```

### For Production Monitoring

1. **Deploy to devnet**

2. **Run typical transactions**

3. **Profile each transaction:**
   ```bash
   ./scripts/profile-transaction.sh <sig> devnet
   ```

4. **Track over time:**
   - Save signatures
   - Re-profile periodically
   - Compare with benchmarks

### For CI/CD

Add to `.github/workflows/test.yml`:

```yaml
- name: Run Tests with Profiling
  run: anchor test

- name: Generate Benchmark Report
  run: |
    node -e "const { generateReport } = require('./tests/utils/compute-profiler'); generateReport();"

- name: Check Compute Limits
  run: |
    if grep -q "❌" target/benchmark/compute-report.md; then
      echo "Error: Instructions exceeding compute limits"
      cat target/benchmark/compute-report.md
      exit 1
    fi

- name: Upload Benchmark Report
  uses: actions/upload-artifact@v3
  with:
    name: compute-benchmarks
    path: target/benchmark/
```

## Current Benchmarks

See [`target/benchmark/compute-units.md`](../target/benchmark/compute-units.md) for latest estimates.

### skill_treasury
- All instructions: < 30,000 CU ✅
- Most expensive: `redeem_skill_for_sol` (~28,000 CU)

### skill_escrow
- All instructions: < 55,000 CU
- Most expensive: `settle_with_signatures` (~52,000 CU) ⚠️
  - Due to Ed25519 signature verification (2 signatures × ~35,000 CU)

### ttt_onchain
- All instructions: < 50,000 CU ✅
- Most expensive: `resolve_if_complete` (~48,000 CU)
  - Includes CPI to escrow settlement

## Optimization Tips

### If Compute Units Are Too High

1. **Minimize Account Deserialization**
   - Use zero-copy deserialization for large accounts
   - Access only needed fields

2. **Optimize Loops**
   - Minimize iterations
   - Use efficient algorithms
   - Consider breaking into multiple instructions

3. **Reduce CPI Calls**
   - Combine operations where possible
   - Cache CPI results if reused

4. **Use Efficient Data Structures**
   - Prefer fixed-size arrays over vectors
   - Use borsh for efficient serialization

5. **Request Compute Budget**
   ```typescript
   import { ComputeBudgetProgram } from "@solana/web3.js";

   const modifyComputeUnits = ComputeBudgetProgram.setComputeUnitLimit({
     units: 300000, // Request more than default 200,000
   });

   const tx = new Transaction()
     .add(modifyComputeUnits)
     .add(yourInstruction);
   ```

   **Note:** This increases cost proportionally.

6. **Profile with solana-program-test**
   ```rust
   #[cfg(test)]
   use solana_program_test::*;

   #[tokio::test]
   async fn test_compute_units() {
       let mut banks_client = /* ... */;

       // Enable logging
       solana_logger::setup_with_default("solana_runtime::message_processor=debug");

       // Execute instruction
       // Check logs for "Consumed X of Y compute units"
   }
   ```

## Compute Unit Costs Reference

### Common Operations

| Operation | Cost (CU) |
|-----------|-----------|
| Account deserialization (small) | ~500 CU |
| Account deserialization (large) | ~2,000 CU |
| SHA256 hash | ~400 CU |
| Ed25519 signature verify | ~35,000 CU |
| Secp256k1 signature verify | ~30,000 CU |
| CPI call (overhead) | ~5,000 CU |
| Token transfer (SPL) | ~15,000 CU |
| Create account | ~8,000 CU |
| System transfer | ~2,000 CU |
| Log message | ~100 CU |

### Program-Specific

| Instruction | Typical Cost |
|-------------|--------------|
| Initialize (small state) | ~10,000 CU |
| Initialize (large state) | ~20,000 CU |
| Simple state update | ~5,000 CU |
| Complex business logic | ~15,000-40,000 CU |
| CPI + settlement | ~40,000-60,000 CU |

## Monitoring in Production

### Solana Explorer

1. Find your transaction
2. View "Compute Units Consumed"
3. Compare to benchmarks

### Custom Monitoring

```typescript
// Add to your backend
async function monitorComputeUsage(signature: string) {
  const tx = await connection.getTransaction(signature, {
    maxSupportedTransactionVersion: 0,
  });

  const consumed = tx?.meta?.computeUnitsConsumed || 0;

  // Log to monitoring service
  logger.info({
    signature,
    computeUnits: consumed,
    percentage: (consumed / 200000) * 100,
  });

  // Alert if high
  if (consumed > 100000) {
    alerting.send({
      level: "warning",
      message: `High compute usage: ${consumed} CU`,
      signature,
    });
  }
}
```

## Troubleshooting

### "Transaction exceeds compute budget"

**Cause:** Instruction uses > 200,000 CU

**Solutions:**
1. Optimize instruction logic
2. Split into multiple instructions
3. Request higher compute budget (costs more)

### Benchmarks show high variance

**Cause:** Compute usage depends on data

**Solutions:**
1. Test with realistic data sizes
2. Profile worst-case scenarios
3. Set conservative limits

### Can't measure in tests

**Cause:** Transaction not found or too fast

**Solutions:**
1. Add `commitment: "confirmed"` to RPC calls
2. Wait for confirmation before measuring
3. Use `connection.confirmTransaction()` first

## Resources

- [Solana Compute Budget](https://docs.solana.com/developing/programming-model/runtime#compute-budget)
- [Optimizing Programs](https://docs.solana.com/developing/on-chain-programs/examples#optimizing-compute)
- [Profiling Tools](https://docs.solana.com/developing/on-chain-programs/debugging#profiling)

## Next Steps

1. ✅ Run `./scripts/benchmark-compute.sh`
2. ✅ Add profiling to critical tests
3. ✅ Set up CI/CD compute limit checks
4. ✅ Monitor production transactions
5. ✅ Optimize high-CU instructions
