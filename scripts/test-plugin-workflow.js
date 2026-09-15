#!/usr/bin/env node
import { execSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import os from "node:os";

console.log("\x1b[36m=== Testing Plugin Development Workflow ===\x1b[0m\n");

const testDir = path.join(os.tmpdir(), `nicle-test-wf-${Date.now()}`);
fs.mkdirSync(testDir, { recursive: true });

function run(cmd) {
  return execSync(cmd, { encoding: "utf-8", stdio: "pipe" });
}

let allPassed = true;

try {
  // Test 1: Scaffold Claude Code CLI plugin with auto-link
  console.log("1. Testing Scaffolding of CLI Assistant ('claude-code-bun')...");
  const cliDir = path.join(testDir, "test-assistant");
  run(`node scripts/create-plugin.js test-assistant --template claude-code-bun --dir "${cliDir}" --link`);
  if (!fs.existsSync(path.join(cliDir, "nicle-plugin.json"))) {
    throw new Error("CLI plugin manifest not found!");
  }
  const cliVal = run(`node scripts/validate-plugin.js "${cliDir}"`);
  console.log("   ✔ Manifest created and validated.");

  // Test 2: Scaffold Companion Service plugin
  console.log("\n2. Testing Scaffolding of Companion Service ('dev-companion-bun')...");
  const srvDir = path.join(testDir, "test-companion");
  run(`node scripts/create-plugin.js test-companion --template dev-companion-bun --dir "${srvDir}"`);
  if (!fs.existsSync(path.join(srvDir, "server.ts"))) {
    throw new Error("Service entry server.ts not found!");
  }
  const srvVal = run(`node scripts/validate-plugin.js "${srvDir}"`);
  console.log("   ✔ Service files created and validated.");

  // Test 3: Scaffold Command Suite plugin
  console.log("\n3. Testing Scaffolding of Command Suite ('command-suite-bun')...");
  const suiteDir = path.join(testDir, "test-suite");
  run(`node scripts/create-plugin.js test-suite --template command-suite-bun --dir "${suiteDir}"`);
  if (!fs.existsSync(path.join(suiteDir, "suite.ts"))) {
    throw new Error("Suite runner suite.ts not found!");
  }
  const suiteVal = run(`node scripts/validate-plugin.js "${suiteDir}"`);
  console.log("   ✔ Command Suite files created and validated.");

  // Test 4: Scaffold AI Harness Agent plugin
  console.log("\n4. Testing Scaffolding of AI Harness Agent ('ai-harness-agent')...");
  const harnessDir = path.join(testDir, "test-harness");
  run(`node scripts/create-plugin.js test-harness --template ai-harness-agent --dir "${harnessDir}"`);
  if (!fs.existsSync(path.join(harnessDir, "nicle-plugin.json"))) {
    throw new Error("AI Harness manifest not found!");
  }
  const harnessVal = run(`node scripts/validate-plugin.js "${harnessDir}"`);
  console.log("   ✔ AI Harness files created and validated.");

  // Test 5: Scaffold Discord Rich Presence Companion
  console.log("\n5. Testing Scaffolding of Discord Rich Presence ('discord-presence-bun')...");
  const discordDir = path.join(testDir, "test-discord");
  run(`node scripts/create-plugin.js test-discord --template discord-presence-bun --dir "${discordDir}"`);
  if (!fs.existsSync(path.join(discordDir, "server.js"))) {
    throw new Error("Discord Presence server.js not found!");
  }
  const discordVal = run(`node scripts/validate-plugin.js "${discordDir}"`);
  console.log("   ✔ Discord Presence files created and validated.");

  // Test 6: Validator catches errors on corrupted manifest
  console.log("\n6. Testing Validator Error Handling on Invalid Manifest...");
  const badDir = path.join(testDir, "bad-plugin");
  fs.mkdirSync(badDir, { recursive: true });
  fs.writeFileSync(path.join(badDir, "nicle-plugin.json"), JSON.stringify({
    id: "invalid id with spaces!",
    name: "",
    pluginType: "invalid-type"
  }));
  let caught = false;
  try {
    run(`node scripts/validate-plugin.js "${badDir}"`);
  } catch {
    caught = true;
  }
  if (!caught) {
    throw new Error("Validator should have failed on invalid manifest!");
  }
  console.log("   ✔ Validator properly caught corrupted manifest.");

} catch (err) {
  console.error("\x1b[31mWorkflow test failed:\x1b[0m", err.message || err);
  allPassed = false;
} finally {
  // Clean up
  try {
    fs.rmSync(testDir, { recursive: true, force: true });
    // Clean up test-assistant from installed.json if linked
    const installedPath = path.join(
      process.env.HOME || "",
      ".local",
      "share",
      "dev.nicle.editor",
      "plugins",
      "installed.json"
    );
    if (fs.existsSync(installedPath)) {
      const records = JSON.parse(fs.readFileSync(installedPath, "utf-8"));
      const filtered = records.filter(r => r.id !== "test-assistant");
      fs.writeFileSync(installedPath, JSON.stringify(filtered, null, 2) + "\n");
    }
  } catch {}
}

if (allPassed) {
  console.log("\n\x1b[32m✔ All plugin development workflow tests PASSED successfully!\x1b[0m\n");
  process.exit(0);
} else {
  process.exit(1);
}
