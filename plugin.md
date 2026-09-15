# Nicle plugin marketplace

Status: product, design, and implementation specification. No plugin is installed and no application code is changed by this document.

## 1. Goal

Add a Plugins page where users discover supported apps, install them, and turn them on or off inside Nicle. The first supported app is **9Router**, distributed through npm and powered by Node.js.

The main workflow is: **Plugins → find 9Router → Install → turn On → Open dashboard → turn Off**.

For this first version, a plugin is a managed local companion app. It runs as a separate process; it does not inject arbitrary JavaScript into the editor. Installing an ordinary npm library does not automatically make it a compatible plugin. Each marketplace app needs a tested Nicle adapter describing installation, startup, readiness, shutdown, and configuration.

This is a user-requested extension beyond the current MVP, which excludes extensions and AI integrations. Before implementing it, update `PRD.md` to include this bounded companion-app feature. Keep the existing editor workflows intact.

## 2. What Install, On, and Off mean

| Action | Behavior |
| --- | --- |
| Install | Download a selected version into Nicle-managed app storage. Leave the app Off. |
| Turn On | Start the installed app, check readiness, and show Running only after it is ready. |
| Turn Off | Stop the process and its owned descendants. Keep the package and settings installed. |
| Open dashboard | Open the verified local app URL in the system browser. Available only when Running. |
| Restart | Stop the owned instance, then start it again with the current configuration. |
| Update | Install a selected newer version with progress and rollback on failure. |
| Uninstall | Stop the app and remove its managed installation. Keep app data unless explicitly selected for removal. |

Turning Off must actually stop the application, rather than merely hide its interface. Show a separate status label beside the switch so On is not confused with Starting or Failed.

Apps are installed per operating-system user, shared across Nicle workspaces. Switching folders does not stop them. Closing Nicle stops all Nicle-owned apps. An optional **Start with Nicle** setting defaults to Off; only apps with that setting explicitly enabled start on the next launch. Persist that preference separately from observed runtime state.

## 3. Marketplace page

Add a Plugins entry to the main navigation and an **Open Plugins** command in the command palette. Open the page in the main workbench while preserving editor tabs and unsaved documents.

Follow `design.md`: graphite surfaces, cyan actions, compact rows, readable labels, and no purple-family colors. Use the existing typography, spacing, focus, and dialog tokens. Do not show fake ratings, downloads, publishers, or app listings to make the marketplace look full.

### Layout

```text
+---------------------------------------------------------------------+
| Plugins                                    [ Search apps…         ] |
| Discover    Installed                                               |
+-------------------------------------+-------------------------------+
| 9Router                             | 9Router                       |
| Local AI routing app                | Overview  Settings  Logs      |
| npm · Node.js required              |                               |
| [Install]                           | Package: 9router              |
|                                     | Version: selected release     |
|                                     | Runtime: Node.js              |
|                                     | Requirements and app access   |
|                                     | [Install]                     |
+-------------------------------------+-------------------------------+
```

After installation, replace Install with a labelled **Off / On** switch and status. The details panel exposes **Open dashboard**, **Restart**, and an overflow menu with **Update** and **Uninstall** when applicable.

- **Discover:** searchable curated catalog with name, purpose, package source, runtime requirement, and install status.
- **Installed:** installed apps, switch, actual status, installed version, and update availability when checked.
- **Details / Overview:** description, publisher metadata from a verified source, package name, version, official links, requirements, and app access information.
- **Details / Settings:** supported port configuration and Start with Nicle. Provider accounts and credentials remain in 9Router's own dashboard initially.
- **Details / Logs:** bounded installation and runtime logs, with phase labels, timestamps, Copy, and Clear view. Clearing logs does not stop the process.

At widths below 900px, use a single-column list; selecting an app opens its details with a Back action. At 375px, stack actions and allow labels to wrap. Use at least 44px switch targets for coarse pointers. All actions remain available from keyboard navigation.

### Empty and error states

| Situation | Message and action |
| --- | --- |
| Nothing installed | “No apps installed yet.” → Browse apps |
| No search matches | “No apps match this search.” → Clear search |
| Catalog unavailable | “Could not load apps.” → Retry; installed apps remain manageable |
| Node.js or npm missing | “Install Node.js and npm to use this app.” → Setup guide and Check again |
| Unsupported runtime version | Show detected and required versions; block installation/start |
| Installation failure | Show concise cause, View logs, and Retry |
| Port conflict | “This port is already in use.” → Choose another port |
| Startup failure | “App could not start.” → View logs and Retry |
| Stop failure | “App has not stopped.” → View logs and Retry stop |
| App exits unexpectedly | “App stopped unexpectedly.” → View logs and Restart |

Nniever kill an unrelated process to resolve a port conflict. Never display Running based only on a spawned PID.

## 4. 9Router integration

### Documentation findings

Context7 documentation retrieved on 2026-09-10 identifies the project as `decolua/9router` and the npm package as `9router`. The CLI documentation describes global npm installation or npx execution, plus `--port`, `--no-browser`, and `--skip-update`. See the [9Router CLI documentation](https://github.com/decolua/9router/blob/master/cli/README.md).

The indexed localhost deployment guide lists Node.js 20+ and npm 9+ requirements. Treat these as documentation findings, then verify the selected package version's actual runtime requirements before release. See the [localhost deployment guide](https://github.com/decolua/9router/blob/master/gitbook/content/en/deployment/localhost.md).

The indexed integration guide references a `/health` endpoint on port 20128. Other indexed pages describe differing dashboard ports. Do not assume one default URL works for every release; validate the selected version's health endpoint, dashboard URL, bind address, and process behavior. See the [integration guide](https://github.com/decolua/9router/blob/master/gitbook/content/en/integration/other-tools.md).

### Nicle adapter requirements

- Pin an explicitly tested package version. Show that version before installation and retain it in the installed record.
- Install into a private Nicle-managed package directory, separate from the user's project and global npm packages. Resolve the installed package's declared executable rather than assuming a platform-specific launcher path.
- Launch the installed executable directly through the validated runtime. Do not use npx for each start because launching must not trigger an implicit download or version change.
- Candidate startup arguments are `--port <configured-port> --no-browser --skip-update`, subject to verification against the pinned release. Nicle owns update checks and the Open dashboard action.
- Prefer port 20128 only after checking the version-specific adapter contract. Persist user overrides and reject unavailable ports with an actionable message.
- Require loopback-only listening for the managed service. Verify a supported bind mechanism; do not invent a `--host` flag. If the selected release cannot meet this requirement, mark that release unsupported until the adapter has a verified solution.
- Confirm that the launcher remains supervised or provides a reliable way to identify and stop its server. A launcher that exits while leaving an untracked daemon is not a working adapter.
- Validate readiness against the owned process and expected service identity, not an arbitrary successful response from an occupied port.
- Map configuration and data locations explicitly. Do not claim settings are isolated unless the pinned release supports that isolation.
- Open the local dashboard in the system browser. Provider setup and account authentication happen there; enabling 9Router does not by itself connect Nicle's editor to an AI model.

Before listing 9Router as installable, verify installation, startup, readiness, dashboard opening, shutdown, and preservation of settings for the pinned release on each supported operating system. Documentation lookup is not runtime verification.

## 5. Lifecycle and persistence

| State | Available primary action | Transition |
| --- | --- | --- |
| Not installed | Install | Installing |
| Installing | Progress / cancel where safely supported | Installed Off, or installation error |
| Installed Off | Turn On | Starting |
| Starting | Starting indicator | Running, or startup error |
| Running | Turn Off | Stopping |
| Stopping | Stopping indicator | Installed Off, or stop error |
| Failed / unexpectedly stopped | Retry or View logs | Starting if retrying |
| Updating | Progress | Prior working version or updated version |
| Uninstalling | Progress | Not installed, or actionable removal error |

Disable conflicting operations while a transition is active. Serialize operations per app so repeated clicks cannot launch duplicate instances. Expose transition state to assistive technology. An unsuccessful stop must retain process ownership and a Stop action; it must not become an ordinary Off state.

Keep these concerns separate in storage:

- **Catalog entry:** stable ID, display name, verified source links, npm package, supported pinned versions, runtime/OS requirements, adapter identifier, and declared access.
- **Installed record:** app ID, exact package version, installation path, integrity information, install timestamp, configuration, and Start with Nicle preference.
- **Runtime state:** current session's process ownership, phase, readiness, local URL, and last error. Never trust a persisted PID after restart without validating ownership and process identity.

Use atomic state writes. Stage installs and updates before replacing a working version. Cancelled or failed installation cleans staging files and never marks an incomplete package Installed. Keep compatible app data outside replaceable package directories.

Updating a running app discloses that it will restart. Stop the owned instance, stage and verify the candidate, switch versions, and start it if it was previously running. Preserve the old version until the new one passes readiness. Back up affected configuration before migrations; do not promise rollback when the app performs irreversible data migrations without a verified recovery path.

On exit, request graceful shutdown, wait a bounded period, then terminate remaining owned descendants if necessary. Do not use broad process-name matching. Coordinate multiple Nicle windows with one app supervisor and an ownership lock so the same app cannot start twice.

## 6. Installation and app access

Show an install review with app identity, package, exact version, runtime requirements, storage location, expected network access, and whether install scripts are required. The user's Install action starts that reviewed installation. Do not request administrator access or modify the workspace's package manifest.

Installed npm apps execute with the user's operating-system permissions. A private installation directory is storage organization, not a security sandbox. Explain this in app access details; do not present permission switches that Nicle cannot enforce.

The initial catalog is curated and shipped with Nicle. Do not accept arbitrary package URLs, Git repositories, tarballs, or custom shell commands in the first version. Each adapter must declare required install scripts; block them by default unless that adapter's reviewed install path requires them. Registry integrity checks detect corruption but do not establish that a publisher or package is trustworthy.

Use structured executable arguments, validated app IDs, canonical managed paths, and allowlisted operations. Descriptions are untrusted text and must not become executable HTML. Installed apps never receive Tauri bridge access through embedded remote content.

Keep secrets out of plugin metadata, telemetry, command-line arguments, and copied logs where known. Redact recognized credentials in captured logs; do not claim arbitrary third-party output can always be sanitized. Store credentials through an appropriate secure facility if Nicle later owns them. The initial integration leaves provider credentials with 9Router.

Uninstall only removes the selected app's managed files. The confirmation offers a separate, unchecked **Also delete app data** option only for data paths Nicle can prove belong exclusively to this installation. Do not delete shared or externally created 9Router settings.

## 7. Architecture and implementation boundaries

Follow the current architecture: Rust owns operating-system work and supervision; Svelte owns presentation. This feature does not require changing frameworks.

| Proposed area | Responsibility |
| --- | --- |
| `src/components/plugins/PluginsPage.svelte` | Discover / Installed navigation, search, selection |
| `src/components/plugins/PluginDetails.svelte` | Overview, settings, lifecycle controls, logs |
| `src/lib/plugins.svelte.ts` | Frontend state and backend event subscriptions |
| `src-tauri/src/plugins/` | Catalog validation, installation, persistence, runtime supervision, adapters |
| `src-tauri/src/plugins/adapters/` | Tested package-specific launch/readiness/data contracts |
| Existing app shell and palette | Plugins entry, Open Plugins command, return to editor |
| Existing process and shutdown code | Integrate plugin-owned process cleanup |

These paths are proposed new modules, not existing functionality. The current process helper offers forceful tree termination; a managed-app lifecycle also needs graceful shutdown and explicit ownership tracking.

Backend operations cover list catalog, list installed, check runtime, install, start, stop, restart, update, uninstall, save app settings, and retrieve bounded logs. Frontend requests refer to app IDs and validated settings, never arbitrary executable strings. Emit progress and state changes from the backend as the source of truth.

Keep installation and probes off the UI thread. Bound concurrent installations, readiness timeouts, and log buffers. Retain at most 1 MiB of in-memory logs per app with an explicit truncation notice. Opening Plugins must not scan the workspace or start installed apps. Offline users can still start and stop compatible installed apps; network-dependent app features may report their own errors.

## 8. Delivery order

1. Update the MVP scope and prove the pinned 9Router adapter with a minimal native lifecycle driver.
2. Build catalog, private installation, installed-state persistence, and failure cleanup.
3. Build start/stop supervision, readiness, logs, and application-exit cleanup.
4. Build Discover, Installed, app details, and accessible on/off controls using `design.md`.
5. Add validated configuration, dashboard opening, updates, and uninstall behavior.
6. Exercise the complete marketplace workflow in the native app and record evidence.

Future scope: arbitrary npm apps after compatibility validation, community submissions, verified publisher workflows, additional runtimes, and editor extension APIs. None is required for the first 9Router marketplace integration.

## 9. Acceptance checklist

- [ ] Plugins is reachable through navigation and the command palette without losing editor state.
- [ ] Discover contains a genuine 9Router entry with package identity and a tested version.
- [ ] Missing or incompatible Node.js/npm produces a clear setup state.
- [ ] Install completes into managed storage without changing global npm packages or the workspace.
- [ ] Installed apps start Off. Switching On produces Starting, then Running only after verified readiness.
- [ ] Open dashboard opens the correct local page and preserves the IDE session.
- [ ] Switching Off stops the owned server and descendants while preserving installation and configuration.
- [ ] Switching On again restarts the same installed version with retained settings.
- [ ] Duplicate clicks/windows do not create duplicate app processes.
- [ ] Port conflicts leave unrelated processes untouched; start/stop failures remain actionable.
- [ ] Closing Plugins or switching workspace leaves a running app alone; exiting Nicle stops owned apps.
- [ ] Relaunch honors only the explicit Start with Nicle preference.
- [ ] Failed installs and updates retain consistent state; update recovery is exercised.
- [ ] Uninstall removes only owned installation files and preserves app data by default.
- [ ] Logs remain bounded, selectable, and useful; known secrets are redacted.
- [ ] The complete page supports keyboard navigation, visible focus, screen-reader status, and narrow layouts.
- [ ] All Nicle-owned plugin surfaces use the dark theme and no purple-family colors.
- [ ] Native installation and lifecycle checks run on every declared supported OS; unverified platforms remain labelled unsupported.

This document defines the intended feature. No installation, lifecycle, security, accessibility, or visual checks are claimed as passed yet.
