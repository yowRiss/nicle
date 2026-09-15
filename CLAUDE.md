# Claude Code Guide for Nicle IDE

This repository is equipped with the **Nicle AI Harness**, allowing Claude Code to orchestrate specialized subagents (such as **Agent A for coding** and **Agent B for reviewing**).

---

## 1. Subagent Orchestration Workflow

When modifying code in this repository, Claude should follow the two-pass subagent harness:

1. **Pass 1: Agent A (Coder Persona)**
   - Implement the feature/fix with minimal, robust code.
   - Adhere strictly to `PRD.md` (no scope bloat).
   - Rust: Zero `.unwrap()` or `.expect()`. Handle all errors via `Result`.
   - Svelte 5: Use runes (`$state`, `$derived`, `$props`). Never store editor document content in global reactive state.
2. **Pass 2: Agent B (Reviewer Persona)**
   - Run `git diff` to inspect modified code.
   - Run `npm run harness:review` to execute the automated review suite.
   - Check for memory bounds (max 1 MiB logs, bounded file reads).
   - Ensure all child processes, PTY sessions, and servers are cleanly terminated on exit.
3. **Pass 3: Agent C (Verification)**
   - Run `npm run check` (`svelte-check`).
   - Run `npm run test:plugins` for plugin-related changes.

---

## 2. Key Commands for Claude Code

- **Automated AI Review**: `npm run harness:review`
- **Verify Full Project**: `npm run harness:verify`
- **Check Svelte Types**: `npm run check`
- **Build Frontend**: `npm run build`
- **Validate Plugin Schema**: `npm run validate-plugin`
- **Test Plugins**: `npm run test:plugins`

---

## 3. Reference Skills & Guidelines

- **AI Harness Skill**: [`.claude/skills/ai-harness/SKILL.md`](.claude/skills/ai-harness/SKILL.md)
- **Rust Guidelines**: [`.claude/skills/rust-skills/SKILL.md`](.claude/skills/rust-skills/SKILL.md)
- **Plugin Development**: [`DEVELOPING_PLUGINS.md`](DEVELOPING_PLUGINS.md)
- **UI Design System**: [`design.md`](design.md)
