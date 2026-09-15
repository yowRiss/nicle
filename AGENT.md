# Working on Nicle

- Follow `PRD.md`; do not expand scope.
- Performance and simplicity first. Avoid unnecessary dependencies.
- Rust owns operating-system work; Svelte owns UI.
- Never scan the workspace at open; load folders lazily and search on demand.
- Keep blocking work off the UI thread and editor contents out of global reactive state.
- Bound large listings, search results, output, and file reads.
- Clean up all child processes, terminals, and preview servers on close/exit.
- Keep the application runnable. Run builds, type checks, and real usage checks after major steps.
- Do not add LSP, Git, in-editor extensions, debugger, or in-editor AI unless `PRD.md` changes. Managed companion apps follow `plugin.md`.

## Multi-Agent AI Harness
Nicle supports and recommends multi-agent task decomposition:
- **Agent A (Coder)**: Implements changes adhering to `PRD.md`, `rust-skills`, and Svelte 5 runes. Zero `.unwrap()`.
- **Agent B (Reviewer)**: Rigorously audits code diffs for safety, memory bounds, and regressions before approval.
- **Agent C (Verifier)**: Runs `npm run check`, `npm run build`, and `npm run harness:verify`.
- Use the **`ai-harness`** skill (`.agents/skills/ai-harness/SKILL.md`) and universal instructions in `AGENTS.md` for role specifications and prompt envelopes.
