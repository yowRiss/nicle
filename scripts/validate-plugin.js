#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const projectRoot = path.resolve(__dirname, "..");

function printUsage() {
  console.log(`
\x1b[36mNicle Plugin Validator\x1b[0m
Usage:
  npm run validate-plugin [plugin-directory-or-manifest]

If no directory is specified, all plugins in examples/ and templates/ are validated.

Examples:
  npm run validate-plugin
  npm run validate-plugin -- ./examples/sample-claude
  npm run validate-plugin -- ./examples/sample-claude/nicle-plugin.json
`);
}

export function validatePlugin(target) {
  let targetPath = path.resolve(process.cwd(), target);
  let manifestPath = targetPath;
  let pluginDir = targetPath;

  if (fs.existsSync(targetPath) && fs.statSync(targetPath).isDirectory()) {
    manifestPath = path.join(targetPath, "nicle-plugin.json");
    pluginDir = targetPath;
  } else {
    pluginDir = path.dirname(targetPath);
  }

  const errors = [];
  const warnings = [];
  const checks = [];

  function pass(msg) {
    checks.push({ status: "pass", msg });
  }
  function fail(msg) {
    errors.push(msg);
    checks.push({ status: "fail", msg });
  }
  function warn(msg) {
    warnings.push(msg);
    checks.push({ status: "warn", msg });
  }

  if (!fs.existsSync(manifestPath)) {
    fail(`Manifest file 'nicle-plugin.json' does not exist at ${manifestPath}`);
    return { ok: false, errors, warnings, checks, manifestPath };
  }
  pass("nicle-plugin.json file exists");

  let manifest;
  try {
    const raw = fs.readFileSync(manifestPath, "utf-8");
    manifest = JSON.parse(raw);
    pass("nicle-plugin.json parses as valid JSON");
  } catch (e) {
    fail(`Failed to parse JSON: ${e.message}`);
    return { ok: false, errors, warnings, checks, manifestPath };
  }

  // ID validation
  if (!manifest.id || typeof manifest.id !== "string" || !manifest.id.trim()) {
    fail("Plugin 'id' is required and must be a non-empty string");
  } else if (!/^[a-zA-Z0-9._-]+$/.test(manifest.id)) {
    fail(`Plugin 'id' ('${manifest.id}') contains invalid characters. Use letters, numbers, '.', '_', or '-'`);
  } else {
    pass(`Valid plugin ID: '${manifest.id}'`);
  }

  // Name validation
  if (!manifest.name || typeof manifest.name !== "string" || !manifest.name.trim()) {
    fail("Plugin 'name' is required and must be a non-empty string");
  } else {
    pass(`Valid plugin name: '${manifest.name}'`);
  }

  // Version validation
  if (!manifest.version || typeof manifest.version !== "string") {
    fail("Plugin 'version' is required (e.g. '1.0.0')");
  } else if (!/^\d+\.\d+\.\d+/.test(manifest.version)) {
    warn(`Plugin version '${manifest.version}' is not strictly semantic versioning (x.y.z)`);
  } else {
    pass(`Valid version: '${manifest.version}'`);
  }

  // PluginType validation
  const validTypes = ["service", "cli", "command"];
  if (!manifest.pluginType || !validTypes.includes(manifest.pluginType)) {
    fail(`'pluginType' must be one of: ${validTypes.join(", ")}`);
  } else {
    pass(`Valid pluginType: '${manifest.pluginType}'`);
  }

  // Runtime validation
  const validRuntimes = ["node", "bun", "python", "binary", "system"];
  if (manifest.runtime) {
    if (!validRuntimes.includes(manifest.runtime.type)) {
      fail(`'runtime.type' must be one of: ${validRuntimes.join(", ")}`);
    } else {
      pass(`Declared runtime: '${manifest.runtime.type}'`);
    }
  }

  // Service configuration
  if (manifest.pluginType === "service") {
    if (!manifest.service || typeof manifest.service !== "object") {
      fail("Service plugins must include a 'service' configuration object");
    } else {
      if (!manifest.service.entry || typeof manifest.service.entry !== "string") {
        fail("service.entry is required for service plugins");
      } else {
        const entryPath = path.join(pluginDir, manifest.service.entry);
        if (!fs.existsSync(entryPath)) {
          fail(`service.entry file '${manifest.service.entry}' was not found on disk at: ${entryPath}`);
        } else {
          pass(`service.entry exists: '${manifest.service.entry}'`);
        }
      }

      if (manifest.service.defaultPort !== undefined) {
        if (typeof manifest.service.defaultPort !== "number" || manifest.service.defaultPort < 1024 || manifest.service.defaultPort > 65535) {
          fail("service.defaultPort must be a valid non-privileged TCP port (1024-65535)");
        } else {
          pass(`service.defaultPort configured: ${manifest.service.defaultPort}`);
        }
      }

      if (manifest.service.readinessPath !== undefined) {
        if (typeof manifest.service.readinessPath !== "string" || !manifest.service.readinessPath.startsWith("/")) {
          fail("service.readinessPath must be a string starting with '/' (e.g. '/health')");
        } else {
          pass(`service.readinessPath configured: '${manifest.service.readinessPath}'`);
        }
      }

      if (manifest.service.dashboardPath !== undefined) {
        if (typeof manifest.service.dashboardPath !== "string" || !manifest.service.dashboardPath.startsWith("/")) {
          fail("service.dashboardPath must be a string starting with '/' (e.g. '/')");
        } else {
          pass(`service.dashboardPath configured: '${manifest.service.dashboardPath}'`);
        }
      }

      if (manifest.service.activityPath !== undefined) {
        if (typeof manifest.service.activityPath !== "string" || !manifest.service.activityPath.startsWith("/")) {
          fail("service.activityPath must be a string starting with '/' (e.g. '/activity')");
        } else {
          pass(`service.activityPath configured: '${manifest.service.activityPath}'`);
        }
      }
    }
  }

  // Commands validation
  if (manifest.commands) {
    if (!Array.isArray(manifest.commands)) {
      fail("'commands' must be an array of command objects");
    } else {
      manifest.commands.forEach((cmd, i) => {
        const prefix = `commands[${i}]`;
        if (!cmd.id || typeof cmd.id !== "string") {
          fail(`${prefix}.id is required`);
        }
        if (!cmd.title || typeof cmd.title !== "string") {
          fail(`${prefix}.title is required`);
        }
        if (!cmd.command || typeof cmd.command !== "string") {
          fail(`${prefix}.command is required`);
        }
        if (!["terminal", "run", "service"].includes(cmd.actionType)) {
          fail(`${prefix}.actionType must be 'terminal', 'run', or 'service'`);
        }
      });
      if (manifest.commands.length > 0) {
        pass(`${manifest.commands.length} Command Palette command(s) verified`);
      }
    }
  }

  return { ok: errors.length === 0, errors, warnings, checks, manifestPath };
}

function printResults(checks, errors, warnings, manifestPath) {
  console.log(`\x1b[36mValidating Nicle plugin at:\x1b[0m ${manifestPath}\n`);
  for (const c of checks) {
    if (c.status === "pass") {
      console.log(`  \x1b[32m✔\x1b[0m ${c.msg}`);
    } else if (c.status === "warn") {
      console.log(`  \x1b[33m⚠\x1b[0m ${c.msg}`);
    } else {
      console.log(`  \x1b[31m✖\x1b[0m ${c.msg}`);
    }
  }

  console.log("");
  if (errors.length === 0) {
    console.log(`\x1b[32m✔ All ${checks.filter(c => c.status === "pass").length} checks passed successfully! Plugin is valid.\x1b[0m\n`);
  } else {
    console.log(`\x1b[31m✖ Validation failed with ${errors.length} error(s).\x1b[0m\n`);
  }
}

function main() {
  const args = process.argv.slice(2);
  if (args.includes("--help") || args.includes("-h")) {
    printUsage();
    process.exit(0);
  }

  if (args.length > 0) {
    const res = validatePlugin(args[0]);
    printResults(res.checks, res.errors, res.warnings, res.manifestPath);
    process.exit(res.ok ? 0 : 1);
  }

  // Scan examples/ and templates/
  const searchDirs = [
    path.join(projectRoot, "examples"),
    path.join(projectRoot, "templates"),
  ];

  let totalErrors = 0;
  let pluginsFound = 0;

  for (const dir of searchDirs) {
    if (!fs.existsSync(dir)) continue;
    const entries = fs.readdirSync(dir, { withFileTypes: true });
    for (const ent of entries) {
      if (ent.isDirectory()) {
        const pPath = path.join(dir, ent.name);
        const mPath = path.join(pPath, "nicle-plugin.json");
        if (fs.existsSync(mPath)) {
          pluginsFound++;
          const res = validatePlugin(pPath);
          printResults(res.checks, res.errors, res.warnings, res.manifestPath);
          if (!res.ok) totalErrors += res.errors.length;
        }
      }
    }
  }

  if (pluginsFound === 0) {
    console.log("No plugins found in examples/ or templates/.");
    process.exit(0);
  }

  console.log(`\x1b[36mValidation Summary:\x1b[0m ${pluginsFound} plugin(s) evaluated. Total errors: ${totalErrors}`);
  process.exit(totalErrors === 0 ? 0 : 1);
}

main();
