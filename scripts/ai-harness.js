#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { execSync, spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const projectRoot = path.resolve(__dirname, "..");

function printBanner() {
  console.log(`\x1b[36m
   _  ___     __        ___   ____  __                         
  / |/ (_)___/ /__     / _ | /  _/ / / /__ _____  ___ ___ ___ 
 /    / / __/ / -_)   / __ |_/ /  / _ / _ \`/ __/ / -_|_-<(_-< 
/_/|_/_/\__/_/\__/   /_/ |_/___/ /_//_\_,_/_/    \__/___/___/ 
                                                               
  Nicle Multi-Agent AI Harness CLI\x1b[0m\n`);
}

function runCommand(cmd, cwd = projectRoot) {
  try {
    return execSync(cmd, { cwd, encoding: "utf8", stdio: ["pipe", "pipe", "pipe"] }).trim();
  } catch (err) {
    return null;
  }
}

function collectSourceFiles(dir, fileList = []) {
  if (!fs.existsSync(dir)) return fileList;
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  for (const ent of entries) {
    const full = path.join(dir, ent.name);
    if (ent.isDirectory()) {
      if (["node_modules", "target", "dist", ".git", ".cargo"].includes(ent.name)) continue;
      collectSourceFiles(full, fileList);
    } else if (ent.isFile()) {
      if (ent.name.endsWith(".rs") || ent.name.endsWith(".svelte") || ent.name.endsWith(".ts") || ent.name === "nicle-plugin.json") {
        fileList.push(full);
      }
    }
  }
  return fileList;
}

function runReview(target = null) {
  console.log("\x1b[33m[Agent B: Reviewer Agent] Auditing codebase for safety, memory bounds, and correctness...\x1b[0m\n");

  let filesToAudit = [];

  if (target) {
    const resolved = path.resolve(process.cwd(), target);
    if (fs.existsSync(resolved)) {
      if (fs.statSync(resolved).isDirectory()) {
        filesToAudit = collectSourceFiles(resolved);
      } else {
        filesToAudit = [resolved];
      }
    }
  } else {
    // Check git diff first
    const gitStatus = runCommand("git status --porcelain");
    if (gitStatus) {
      const lines = gitStatus.split("\n").filter(Boolean);
      for (const line of lines) {
        const rel = line.substring(3).trim();
        const p = path.resolve(projectRoot, rel);
        if (fs.existsSync(p) && !fs.statSync(p).isDirectory()) {
          filesToAudit.push(p);
        }
      }
      console.log(`Auditing ${filesToAudit.length} modified git path(s)...`);
    } else {
      // Fallback: audit all project source files in src/ and src-tauri/src/
      console.log("Auditing all source files in src/ and src-tauri/src/...");
      filesToAudit = [
        ...collectSourceFiles(path.join(projectRoot, "src")),
        ...collectSourceFiles(path.join(projectRoot, "src-tauri", "src")),
      ];
    }
  }

  let p0Errors = [];
  let p1Warnings = [];
  let auditedCount = 0;

  for (const fullPath of filesToAudit) {
    const relPath = path.relative(projectRoot, fullPath);
    if (!fs.existsSync(fullPath)) continue;
    auditedCount++;

    const content = fs.readFileSync(fullPath, "utf8");

    // Rust auditing
    if (fullPath.endsWith(".rs")) {
      const linesContent = content.split("\n");
      let inTestModule = false;
      linesContent.forEach((codeLine, idx) => {
        const lineNum = idx + 1;
        const trimmed = codeLine.trim();
        if (trimmed.includes("#[cfg(test)]") || trimmed.startsWith("mod tests") || trimmed.startsWith("#[test]")) {
          inTestModule = true;
        }
        if (inTestModule) return; // Allow unwrap/expect in unit tests
        if (trimmed.startsWith("//") || trimmed.startsWith("/*") || trimmed.startsWith("*")) return;

        if (/\.unwrap\(\)/.test(trimmed)) {
          p0Errors.push(`${relPath}:${lineNum} - Prohibited '.unwrap()' call detected. Clippy denies unwrap. Return Result<T, AppError>.`);
        }
        if (/\.expect\(/.test(trimmed)) {
          p0Errors.push(`${relPath}:${lineNum} - Prohibited '.expect()' call detected. Clippy denies expect. Return Result<T, AppError>.`);
        }
      });
    }

    // Svelte 5 auditing
    if (fullPath.endsWith(".svelte")) {
      const linesContent = content.split("\n");
      linesContent.forEach((codeLine, idx) => {
        const lineNum = idx + 1;
        const trimmed = codeLine.trim();
        // Check legacy reactive declaration $: in Svelte 5
        if (/^\$:\s+/.test(trimmed)) {
          p1Warnings.push(`${relPath}:${lineNum} - Legacy reactive declaration '$:' detected. Use Svelte 5 '$derived' or '$effect' rune instead.`);
        }
      });
    }

    // Plugin manifest validation
    if (path.basename(fullPath) === "nicle-plugin.json") {
      try {
        const manifest = JSON.parse(content);
        if (!manifest.id || !manifest.name || !manifest.version) {
          p0Errors.push(`${relPath} - Missing required manifest fields (id, name, version).`);
        }
      } catch (err) {
        p0Errors.push(`${relPath} - Invalid JSON syntax: ${err.message}`);
      }
    }
  }

  console.log("\n=================== REVIEW REPORT ===================");
  console.log(`Audited ${auditedCount} file(s).`);
  if (p0Errors.length > 0) {
    console.log(`\x1b[31mSTATUS: CHANGES_REQUESTED (P0 Blockers: ${p0Errors.length})\x1b[0m\n`);
    p0Errors.forEach(err => console.log(` \x1b[31m✖ [P0]\x1b[0m ${err}`));
  } else {
    console.log("\x1b[32mSTATUS: APPROVED (0 P0 Blockers)\x1b[0m\n");
  }

  if (p1Warnings.length > 0) {
    console.log(`\x1b[33mWarnings (P1 / P2): ${p1Warnings.length}\x1b[0m`);
    p1Warnings.forEach(w => console.log(` \x1b[33m▲ [P1]\x1b[0m ${w}`));
  }

  console.log("=====================================================\n");
  return p0Errors.length === 0;
}

function runVerify() {
  console.log("\x1b[34m[Agent C: Verifier Agent] Executing automated verification pipeline...\x1b[0m\n");

  console.log("1. Running Svelte & TypeScript check (svelte-check)...");
  const svelteCheck = spawnSync("npm", ["run", "check"], { cwd: projectRoot, stdio: "inherit" });
  if (svelteCheck.status !== 0) {
    console.error("\x1b[31m✖ Svelte check failed!\x1b[0m");
    process.exit(1);
  }
  console.log("\x1b[32m✔ Svelte check passed.\x1b[0m\n");

  console.log("2. Validating sample plugin manifests...");
  const validatePlugins = spawnSync("node", ["scripts/validate-plugin.js"], { cwd: projectRoot, stdio: "inherit" });
  if (validatePlugins.status !== 0) {
    console.error("\x1b[31m✖ Plugin validation failed!\x1b[0m");
    process.exit(1);
  }
  console.log("\x1b[32m✔ Plugin validation passed.\x1b[0m\n");

  console.log("3. Running AI Harness Static Review...");
  const reviewPassed = runReview();
  if (!reviewPassed) {
    console.error("\x1b[31m✖ AI Harness Review identified P0 blockers!\x1b[0m");
    process.exit(1);
  }

  console.log("\x1b[32m🎉 All Verification Checks Passed! Ready for deployment/commit.\x1b[0m\n");
}

function dispatchAgent(role, task) {
  console.log(`\x1b[36mGenerating Task Envelope for Role: [${role.toUpperCase()}]...\x1b[0m\n`);
  
  if (role.toLowerCase().includes("code") || role.toLowerCase() === "a") {
    console.log(`--------------------- COPY TO AGENT A (CODER) ---------------------`);
    console.log(`You are Agent A (Coder / Implementer) for Nicle IDE.
ROLE: Write minimal, robust, high-performance code adhering to PRD.md.
TASK: ${task || "<Specify task here>"}

CONSTRAINTS:
1. Rust: Absolutely NO .unwrap() or .expect(); follow rust-skills; keep memory bounded.
2. Svelte 5: Modern runes ($state, $derived, $effect, $props); keep editor state out of global reactivity.
3. Architecture: Process isolation; clean up child processes on close; loopback-only networking.
4. UI: Graphite dark/light surfaces conforming to design.md (no purple tokens).

DELIVERABLE: Modified files with clear explanations of design decisions.`);
    console.log(`-------------------------------------------------------------------`);
  } else if (role.toLowerCase().includes("review") || role.toLowerCase() === "b") {
    console.log(`-------------------- COPY TO AGENT B (REVIEWER) -------------------`);
    console.log(`You are Agent B (Reviewer / Auditor) for Nicle IDE.
ROLE: Adversarial code review auditing git diffs for correctness, safety, and performance.
TASK TO REVIEW: ${task || "<Specify diff or feature to review>"}

CHECKLIST:
1. Scope: Did Agent A stay within PRD.md boundaries? Any unneeded dependencies added?
2. Rust Safety: Are there any .unwrap() or .expect() calls? Are memory buffers bounded?
3. Svelte 5: Are runes used properly? Is reactive churn minimized?
4. Process Lifecycle: Are all spawned PTYs/daemons cleanly terminated on exit?

OUTPUT FORMAT:
- STATUS: [APPROVED | CHANGES_REQUESTED]
- FINDINGS: List of P0 (Blockers), P1 (Critical), P2 (Moderate), P3 (Minor) issues with line numbers.
- ACTIONABLE REMEDIATION: Exact instructions for Agent A to fix.`);
    console.log(`-------------------------------------------------------------------`);
  } else {
    console.log(`Unknown role '${role}'. Available roles: 'coder' (Agent A), 'reviewer' (Agent B), 'verifier' (Agent C).`);
  }
}

function printUsage() {
  console.log(`Usage:
  node scripts/ai-harness.js <command> [options]

Commands:
  review [path]                  Run automated Agent B static review on diffs or codebase
  verify                         Run complete quality verification (Svelte check, plugins, review)
  dispatch --role <r> --task <t> Generate prompt envelope for Agent A (coder) or Agent B (reviewer)
  help                           Show this help menu

Examples:
  npm run harness:review
  npm run harness:verify
  node scripts/ai-harness.js review src/
  node scripts/ai-harness.js dispatch --role coder --task "Implement auto-save timer"
  node scripts/ai-harness.js dispatch --role reviewer --task "Review plugins.svelte.ts changes"
`);
}

// CLI Arg Parsing
const args = process.argv.slice(2);
const command = args[0] || "help";

printBanner();

switch (command) {
  case "review": {
    const target = args[1] || null;
    const ok = runReview(target);
    process.exit(ok ? 0 : 1);
    break;
  }
  case "verify":
    runVerify();
    break;
  case "dispatch": {
    let role = "coder";
    let task = "";
    for (let i = 1; i < args.length; i++) {
      if (args[i] === "--role" || args[i] === "-r") {
        role = args[++i] || "coder";
      } else if (args[i] === "--task" || args[i] === "-t") {
        task = args[++i] || "";
      }
    }
    dispatchAgent(role, task);
    break;
  }
  case "help":
  case "--help":
  case "-h":
  default:
    printUsage();
    break;
}
