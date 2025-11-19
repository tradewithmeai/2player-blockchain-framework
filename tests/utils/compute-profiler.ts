/**
 * Compute Unit Profiling Utilities
 *
 * Use these helpers in tests to measure and track compute unit usage.
 *
 * Example usage:
 * ```typescript
 * import { measureComputeUnits, logComputeUnits } from './utils/compute-profiler';
 *
 * it("should use reasonable compute units", async () => {
 *   const sig = await program.methods.myInstruction().rpc();
 *   const computeUnits = await measureComputeUnits(provider.connection, sig);
 *
 *   logComputeUnits("myInstruction", computeUnits);
 *
 *   // Assert compute unit limits
 *   expect(computeUnits).toBeLessThan(50000);
 * });
 * ```
 */

import { Connection } from "@solana/web3.js";
import * as fs from "fs";
import * as path from "path";

export interface ComputeUnitResult {
  consumed: number;
  limit: number;
  percentage: number;
  status: "good" | "warning" | "critical";
}

export interface ComputeUnitBenchmark {
  instruction: string;
  program: string;
  consumed: number;
  timestamp: number;
  status: "good" | "warning" | "critical";
}

/**
 * Measure compute units consumed by a transaction
 *
 * @param connection - Solana connection
 * @param signature - Transaction signature
 * @returns Compute unit usage details
 */
export async function measureComputeUnits(
  connection: Connection,
  signature: string
): Promise<ComputeUnitResult> {
  // Fetch transaction with max supported version
  const tx = await connection.getTransaction(signature, {
    commitment: "confirmed",
    maxSupportedTransactionVersion: 0,
  });

  if (!tx || !tx.meta) {
    throw new Error(`Transaction not found or missing metadata: ${signature}`);
  }

  const consumed = tx.meta.computeUnitsConsumed || 0;
  const limit = 200000; // Solana's default compute unit limit
  const percentage = (consumed / limit) * 100;

  let status: "good" | "warning" | "critical";
  if (consumed < 50000) {
    status = "good";
  } else if (consumed < 100000) {
    status = "warning";
  } else {
    status = "critical";
  }

  return { consumed, limit, percentage, status };
}

/**
 * Log compute unit usage to console
 *
 * @param instructionName - Name of the instruction
 * @param result - Compute unit result
 */
export function logComputeUnits(
  instructionName: string,
  result: ComputeUnitResult
): void {
  const statusIcon =
    result.status === "good"
      ? "✅"
      : result.status === "warning"
      ? "⚠️"
      : "❌";

  console.log(
    `${statusIcon} ${instructionName}: ${result.consumed.toLocaleString()} CU (${result.percentage.toFixed(1)}%)`
  );
}

/**
 * Save compute unit benchmark to file
 *
 * Appends benchmark data to target/benchmark/compute-data.json
 *
 * @param program - Program name (e.g., "skill_treasury")
 * @param instruction - Instruction name (e.g., "deposit_sol_and_mint_skill")
 * @param result - Compute unit result
 */
export function saveBenchmark(
  program: string,
  instruction: string,
  result: ComputeUnitResult
): void {
  const benchmark: ComputeUnitBenchmark = {
    instruction,
    program,
    consumed: result.consumed,
    timestamp: Date.now(),
    status: result.status,
  };

  const benchmarkDir = path.join(process.cwd(), "target", "benchmark");
  const benchmarkFile = path.join(benchmarkDir, "compute-data.json");

  // Create directory if needed
  if (!fs.existsSync(benchmarkDir)) {
    fs.mkdirSync(benchmarkDir, { recursive: true });
  }

  // Read existing benchmarks
  let benchmarks: ComputeUnitBenchmark[] = [];
  if (fs.existsSync(benchmarkFile)) {
    const data = fs.readFileSync(benchmarkFile, "utf-8");
    benchmarks = JSON.parse(data);
  }

  // Append new benchmark
  benchmarks.push(benchmark);

  // Write back
  fs.writeFileSync(benchmarkFile, JSON.stringify(benchmarks, null, 2));
}

/**
 * Assert compute unit usage is within limit
 *
 * @param instructionName - Name of the instruction
 * @param result - Compute unit result
 * @param maxAllowed - Maximum allowed compute units (default: 100,000)
 * @throws Error if compute units exceed limit
 */
export function assertComputeLimit(
  instructionName: string,
  result: ComputeUnitResult,
  maxAllowed: number = 100000
): void {
  if (result.consumed > maxAllowed) {
    throw new Error(
      `${instructionName} exceeded compute limit: ${result.consumed} > ${maxAllowed} CU`
    );
  }
}

/**
 * Generate compute unit report from benchmarks
 *
 * Reads target/benchmark/compute-data.json and generates markdown report
 *
 * @param outputPath - Path to save report (default: target/benchmark/compute-report.md)
 */
export function generateReport(
  outputPath: string = "target/benchmark/compute-report.md"
): void {
  const benchmarkFile = path.join(
    process.cwd(),
    "target",
    "benchmark",
    "compute-data.json"
  );

  if (!fs.existsSync(benchmarkFile)) {
    console.log("No benchmark data found. Run tests with compute profiling first.");
    return;
  }

  const benchmarks: ComputeUnitBenchmark[] = JSON.parse(
    fs.readFileSync(benchmarkFile, "utf-8")
  );

  // Group by program
  const byProgram: Record<string, ComputeUnitBenchmark[]> = {};
  benchmarks.forEach((b) => {
    if (!byProgram[b.program]) {
      byProgram[b.program] = [];
    }
    byProgram[b.program].push(b);
  });

  // Generate markdown
  let markdown = "# Compute Unit Benchmark Report\n\n";
  markdown += `Generated: ${new Date().toISOString()}\n\n`;
  markdown += "---\n\n";

  Object.entries(byProgram).forEach(([program, programBenchmarks]) => {
    markdown += `## ${program}\n\n`;
    markdown += "| Instruction | Compute Units | Status |\n";
    markdown += "|------------|---------------|--------|\n";

    // Get latest benchmark for each instruction
    const latestByInstruction: Record<string, ComputeUnitBenchmark> = {};
    programBenchmarks.forEach((b) => {
      if (
        !latestByInstruction[b.instruction] ||
        b.timestamp > latestByInstruction[b.instruction].timestamp
      ) {
        latestByInstruction[b.instruction] = b;
      }
    });

    Object.values(latestByInstruction).forEach((b) => {
      const icon = b.status === "good" ? "✅" : b.status === "warning" ? "⚠️" : "❌";
      markdown += `| \`${b.instruction}\` | ${b.consumed.toLocaleString()} CU | ${icon} |\n`;
    });

    markdown += "\n";
  });

  markdown += "---\n\n";
  markdown += "## Legend\n\n";
  markdown += "- ✅ Good: < 50,000 CU\n";
  markdown += "- ⚠️ Warning: 50,000 - 100,000 CU\n";
  markdown += "- ❌ Critical: > 100,000 CU\n";

  fs.writeFileSync(outputPath, markdown);
  console.log(`📊 Compute unit report generated: ${outputPath}`);
}

/**
 * Example test showing compute profiling usage
 *
 * Copy this into your test files and adapt as needed:
 *
 * ```typescript
 * describe("Compute Unit Profiling", () => {
 *   it("should measure deposit compute units", async () => {
 *     const sig = await program.methods
 *       .depositSolAndMintSkill(new BN(1_000_000))
 *       .accounts({ ... })
 *       .rpc();
 *
 *     const result = await measureComputeUnits(provider.connection, sig);
 *     logComputeUnits("deposit_sol_and_mint_skill", result);
 *     saveBenchmark("skill_treasury", "deposit_sol_and_mint_skill", result);
 *     assertComputeLimit("deposit_sol_and_mint_skill", result, 50000);
 *   });
 * });
 * ```
 */
