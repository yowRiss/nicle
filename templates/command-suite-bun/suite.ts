#!/usr/bin/env bun
import { existsSync, rmSync } from "node:fs";
import { resolve } from "node:path";

const flag = process.argv[2] || "--info";

if (flag === "--info") {
  console.log("\x1b[36m=== Environment Diagnostics ===\x1b[0m");
  console.log(`Node:    ${process.version}`);
  console.log(`Bun:     ${(process.versions as any).bun || "N/A"}`);
  console.log(`Platform:${process.platform} (${process.arch})`);
  console.log(`CWD:     ${process.cwd()}`);
  console.log(`Time:    ${new Date().toISOString()}`);
} else if (flag === "--clean") {
  console.log("\x1b[33mCleaning build artifacts in active workspace...\x1b[0m");
  const targets = ["dist", "build", ".cache", "tmp", "coverage"];
  let cleaned = 0;
  for (const t of targets) {
    const p = resolve(process.cwd(), t);
    if (existsSync(p)) {
      rmSync(p, { recursive: true, force: true });
      console.log(`  ✔ Removed ${t}`);
      cleaned++;
    }
  }
  if (cleaned === 0) {
    console.log("  No artifacts found to clean.");
  } else {
    console.log(`\x1b[32mClean complete: ${cleaned} items removed.\x1b[0m`);
  }
} else {
  console.log(`Unknown flag: ${flag}. Usage: bun run suite.ts [--info | --clean]`);
}
