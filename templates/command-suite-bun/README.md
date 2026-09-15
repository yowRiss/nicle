# Command Suite Plugin for Nicle

This plugin registers workspace utility commands directly into Nicle's Command Palette (`Ctrl+Shift+P`).

## Registered Commands
- **Command Suite: System & Environment Info**: Prints Node, Bun, and OS diagnostics into the terminal.
- **Command Suite: Clean Build Artifacts**: Removes temporary build files from the workspace.
- **Command Suite: Run Project Tests**: Runs project tests via Bun or npm.

## Customizing Commands
Edit `nicle-plugin.json` to add your own terminal or background commands:
```json
{
  "id": "myplugin.mycommand",
  "title": "My Plugin: My Action",
  "description": "Runs in Nicle terminal",
  "actionType": "terminal",
  "command": "bun run my-script.ts"
}
```
