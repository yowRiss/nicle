#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import readline from "node:readline/promises";
import { stdin as input, stdout as output } from "node:process";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const projectRoot = path.resolve(__dirname, "..");
const templatesDir = path.resolve(projectRoot, "templates");

const AVAILABLE_TEMPLATES = {
  "claude-code-bun": {
    name: "CLI Assistant (Claude Code + Bun)",
    desc: "Interactive terminal assistant tool running inside Nicle's PTY",
    type: "cli",
  },
  "dev-companion-bun": {
    name: "Companion Service (Bun HTTP Server)",
    desc: "Background service daemon with loopback port, /health probe, and web dashboard",
    type: "service",
  },
  "command-suite-bun": {
    name: "Command Suite (Bun Workspace Actions)",
    desc: "Action-oriented utility commands registered into Nicle's Command Palette",
    type: "command",
  },
  "ai-harness-agent": {
    name: "AI Harness Agent (Multi-Agent Task Harness)",
    desc: "Coordinates Agent A (Coder) and Agent B (Reviewer) for any LLM (Antigravity, Claude, Codex)",
    type: "command",
  },
  "discord-presence-bun": {
    name: "Discord Rich Presence (RPC Companion)",
    desc: "Companion service displaying live workspace & file editing activity on Discord",
    type: "service",
  },
};

function printUsage() {
  console.log(`
\x1b[36mNicle Plugin Scaffolder\x1b[0m
Usage:
  npm run create-plugin -- [plugin-name] [options]

Templates:
  claude-code-bun       CLI assistant running via Bun in Nicle PTY (default)
  dev-companion-bun     Background companion HTTP service with dashboard
  command-suite-bun     Action-oriented command runner for Command Palette
  ai-harness-agent      AI multi-agent harness (Agent A: Coder, Agent B: Reviewer)
  discord-presence-bun  Discord Rich Presence companion service with dashboard

Options:
  --template, -t <tpl>   Template name (claude-code-bun | dev-companion-bun | command-suite-bun | ai-harness-agent | discord-presence-bun)
  --dir, -d <path>       Output directory (default: ./<plugin-slug>)
  --author, -a <author>  Author or organization name
  --desc <description>   Short plugin summary
  --link, -l             Automatically link plugin to Nicle IDE for live dev
  --interactive, -i      Force interactive prompt mode
  --help, -h             Show this help message

Examples:
  npm run create-plugin -- my-assistant --template claude-code-bun --link
  npm run create-plugin -- my-api --template dev-companion-bun --dir ./plugins/my-api
  npm run create-plugin -- --interactive
`);
}

function getNicleInstalledJsonPath() {
  const home = process.env.HOME || process.env.USERPROFILE || "";
  let base = "";
  if (process.platform === "darwin") {
    base = path.join(home, "Library", "Application Support", "dev.nicle.editor", "plugins");
  } else if (process.platform === "win32") {
    const appdata = process.env.APPDATA || path.join(home, "AppData", "Roaming");
    base = path.join(appdata, "dev.nicle.editor", "plugins");
  } else {
    const dataHome = process.env.XDG_DATA_HOME || path.join(home, ".local", "share");
    base = path.join(dataHome, "dev.nicle.editor", "plugins");
  }
  return path.join(base, "installed.json");
}

function linkPluginDirectly(manifest, pluginDir) {
  const installedPath = getNicleInstalledJsonPath();
  const dir = path.dirname(installedPath);
  if (!fs.existsSync(dir)) {
    fs.mkdirSync(dir, { recursive: true });
  }
  let installed = [];
  if (fs.existsSync(installedPath)) {
    try {
      installed = JSON.parse(fs.readFileSync(installedPath, "utf-8"));
    } catch {
      installed = [];
    }
  }

  const existingIdx = installed.findIndex((r) => r.id === manifest.id);
  const record = {
    id: manifest.id,
    version: manifest.version,
    installPath: pluginDir,
    installedAt: Math.floor(Date.now() / 1000),
    port: manifest.service?.defaultPort || 3000,
    startWithNicle: false,
    isDev: true,
    manifest: manifest,
  };

  if (existingIdx >= 0) {
    installed[existingIdx] = record;
  } else {
    installed.push(record);
  }

  fs.writeFileSync(installedPath, JSON.stringify(installed, null, 2) + "\n");
  return installedPath;
}

function toSlug(str) {
  return str
    .toLowerCase()
    .replace(/[^a-z0-9_-]/g, "-")
    .replace(/-+/g, "-")
    .replace(/^-|-$/g, "");
}

async function main() {
  const rawArgs = process.argv.slice(2);

  if (rawArgs.includes("--help") || rawArgs.includes("-h")) {
    printUsage();
    process.exit(0);
  }

  let pluginName = "";
  let templateName = "";
  let targetDir = "";
  let author = "";
  let description = "";
  let shouldLink = false;
  let interactive = rawArgs.includes("--interactive") || rawArgs.includes("-i");

  for (let i = 0; i < rawArgs.length; i++) {
    const arg = rawArgs[i];
    if (arg === "--template" || arg === "-t") {
      templateName = rawArgs[++i] || "";
    } else if (arg === "--dir" || arg === "-d") {
      targetDir = rawArgs[++i] || "";
    } else if (arg === "--author" || arg === "-a") {
      author = rawArgs[++i] || "";
    } else if (arg === "--desc") {
      description = rawArgs[++i] || "";
    } else if (arg === "--link" || arg === "-l") {
      shouldLink = true;
    } else if (!arg.startsWith("-") && !pluginName) {
      pluginName = arg;
    } else if (!arg.startsWith("-") && pluginName && !templateName) {
      templateName = arg;
    } else if (!arg.startsWith("-") && pluginName && templateName && !targetDir) {
      targetDir = arg;
    }
  }

  // If no plugin name provided and in interactive terminal, prompt user
  if (!pluginName && (interactive || input.isTTY)) {
    console.log("\x1b[36m✨ Welcome to the Nicle Plugin Creator\x1b[0m\n");
    const rl = readline.createInterface({ input, output });

    try {
      while (!pluginName.trim()) {
        pluginName = await rl.question("\x1b[1m? Plugin display name:\x1b[0m ");
      }

      console.log("\n\x1b[1m? Select plugin template:\x1b[0m");
      console.log("  1) \x1b[32mclaude-code-bun\x1b[0m      - Interactive CLI assistant (Bun terminal)");
      console.log("  2) \x1b[34mdev-companion-bun\x1b[0m    - Background companion service (HTTP & dashboard)");
      console.log("  3) \x1b[35mcommand-suite-bun\x1b[0m    - Command Palette action suite (workspace tools)");
      console.log("  4) \x1b[36mdiscord-presence-bun\x1b[0m - Discord Rich Presence companion (RPC status)");

      const choice = await rl.question("  Enter choice [1-4] (default 1): ");
      if (choice.trim() === "2") {
        templateName = "dev-companion-bun";
      } else if (choice.trim() === "3") {
        templateName = "command-suite-bun";
      } else if (choice.trim() === "4") {
        templateName = "discord-presence-bun";
      } else {
        templateName = "claude-code-bun";
      }

      const defaultSlug = toSlug(pluginName);
      const customSlug = await rl.question(`\x1b[1m? Plugin ID slug\x1b[0m [${defaultSlug}]: `);
      if (customSlug.trim()) {
        pluginName = customSlug.trim();
      }

      const descAns = await rl.question("\x1b[1m? Description\x1b[0m: ");
      if (descAns.trim()) description = descAns.trim();

      const authorAns = await rl.question("\x1b[1m? Author\x1b[0m: ");
      if (authorAns.trim()) author = authorAns.trim();

      const defaultDir = `./${toSlug(pluginName)}`;
      const dirAns = await rl.question(`\x1b[1m? Target directory\x1b[0m [${defaultDir}]: `);
      if (dirAns.trim()) targetDir = dirAns.trim();

      const linkAns = await rl.question("\x1b[1m? Link immediately to Nicle for live development? [Y/n]:\x1b[0m ");
      shouldLink = !linkAns.trim().toLowerCase().startsWith("n");
    } finally {
      rl.close();
    }
  }

  if (!pluginName) {
    console.error("\x1b[31mError: Plugin name is required.\x1b[0m");
    printUsage();
    process.exit(1);
  }

  const slug = toSlug(pluginName);
  if (!slug) {
    console.error("\x1b[31mError: Invalid plugin name resulting in empty slug.\x1b[0m");
    process.exit(1);
  }

  // Alias resolution
  if (!templateName || templateName === "claude" || templateName === "cli") {
    templateName = "claude-code-bun";
  } else if (templateName === "service" || templateName === "companion") {
    templateName = "dev-companion-bun";
  } else if (templateName === "command" || templateName === "suite") {
    templateName = "command-suite-bun";
  } else if (templateName === "discord" || templateName === "rpc" || templateName === "presence") {
    templateName = "discord-presence-bun";
  }

  const templatePath = path.join(templatesDir, templateName);
  if (!fs.existsSync(templatePath)) {
    console.error(`\x1b[31mError: Template '${templateName}' not found in ${templatesDir}\x1b[0m`);
    console.log("Available templates:", Object.keys(AVAILABLE_TEMPLATES).join(", "));
    process.exit(1);
  }

  const destination = targetDir ? path.resolve(process.cwd(), targetDir) : path.resolve(process.cwd(), slug);

  if (fs.existsSync(destination) && fs.readdirSync(destination).length > 0) {
    console.error(`\x1b[31mError: Destination directory '${destination}' already exists and is not empty.\x1b[0m`);
    process.exit(1);
  }

  fs.mkdirSync(destination, { recursive: true });

  console.log(`\n\x1b[36m⚡ Scaffolding '${slug}' using template '${templateName}'...\x1b[0m`);

  // Copy files
  const files = fs.readdirSync(templatePath);
  let finalManifest = null;

  for (const file of files) {
    const srcFile = path.join(templatePath, file);
    const destFile = path.join(destination, file);

    if (file === "nicle-plugin.json") {
      const manifest = JSON.parse(fs.readFileSync(srcFile, "utf-8"));
      manifest.id = slug;
      manifest.name = pluginName.length > slug.length ? pluginName : slug.replace(/-/g, " ").replace(/\b\w/g, (l) => l.toUpperCase());
      if (description) manifest.description = description;
      if (author) manifest.author = author;
      if (manifest.commands) {
        manifest.commands = manifest.commands.map((cmd) => ({
          ...cmd,
          id: cmd.id.replace(/^(claude|companion|suite|discord)/, slug),
        }));
      }
      finalManifest = manifest;
      fs.writeFileSync(destFile, JSON.stringify(manifest, null, 2) + "\n");
    } else if (file === "package.json") {
      const pkg = JSON.parse(fs.readFileSync(srcFile, "utf-8"));
      pkg.name = slug;
      if (description) pkg.description = description;
      if (author) pkg.author = author;
      fs.writeFileSync(destFile, JSON.stringify(pkg, null, 2) + "\n");
    } else if (file === "README.md") {
      let content = fs.readFileSync(srcFile, "utf-8");
      content = content.replace(/Claude Code Plugin/g, `${pluginName} Plugin`);
      content = content.replace(/claude-code/g, slug);
      content = content.replace(/dev-companion/g, slug);
      content = content.replace(/command-suite/g, slug);
      content = content.replace(/discord-presence/g, slug);
      fs.writeFileSync(destFile, content);
    } else {
      fs.copyFileSync(srcFile, destFile);
    }
  }

  console.log(`\x1b[32m✔ Plugin files successfully generated at:\x1b[0m ${destination}`);

  // Direct dev link if requested
  if (shouldLink && finalManifest) {
    try {
      const linkPath = linkPluginDirectly(finalManifest, destination);
      console.log(`\x1b[32m✔ Automatically linked to Nicle IDE!\x1b[0m (${linkPath})`);
    } catch (err) {
      console.warn(`\x1b[33m⚠ Could not auto-link directly: ${err.message}\x1b[0m`);
    }
  }

  console.log(`
\x1b[36mNext steps:\x1b[0m
  1. Inspect or customize your plugin:
     \x1b[33mcd "${destination}"\x1b[0m
  2. Open Nicle IDE (\x1b[1mnicle\x1b[0m)
  3. Press \x1b[1mCtrl+Shift+X\x1b[0m to view under \x1b[1mInstalled\x1b[0m with \x1b[32m[DEV]\x1b[0m badge${shouldLink ? "" : " (or click Link Local)"}
  4. Press \x1b[1mCtrl+Shift+P\x1b[0m to test registered commands in the Command Palette!
  5. Validate manifest anytime:
     \x1b[33mnpm run validate-plugin -- "${destination}"\x1b[0m
`);
}

main().catch((err) => {
  console.error("\x1b[31mUnexpected error:\x1b[0m", err);
  process.exit(1);
});
