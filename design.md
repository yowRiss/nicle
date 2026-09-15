# Nicle IDE redesign: dark Material UI

Status: design specification for future implementation. This document does not change the running IDE.

## 1. Atmosphere and identity

Nicle should feel like a complete desktop workbench with a Material UI design language: neutral charcoal surfaces, layered greys, readable code, compact tools, and clear workspace context. Replace the sparse, centered welcome composition with useful actions and organized information. Keep the editor dominant once a file opens.

The signature is a continuous workspace frame: the workspace name, selected file, active editor tab, and active tool panel share a light-grey selection indicator and solid grey selected surfaces. Material-style component hierarchy, consistent shapes, and restrained elevation separate navigation, editing, and output without surrounding everything in cards. Apply this visual language to the existing Svelte components; this specification does not require a React or MUI package migration.

### Non-negotiable direction

- Dark is the default and the visual target for the entire IDE.
- Use dark + grey throughout application chrome: charcoal backgrounds, neutral grey surfaces, off-white text, and light-grey primary actions. Reserve chromatic colors for meaningful status, editor syntax, and terminal output.
- No gradients anywhere in application-owned UI: backgrounds, buttons, borders, text, overlays, selection, or elevated surfaces. Use solid fills, discrete tonal layers, and neutral shadows only.
- No purple, violet, lavender, magenta, or purple-leaning indigo in application chrome, syntax highlighting, icons, selection, focus, terminal palettes, gradients, or shadows.
- Make space useful through navigation, actions, and real context. Do not fill it with invented files, fake activity, decorative charts, or unsupported features.
- Preserve existing editing, filesystem, terminal, run, preview, settings, and keyboard workflows.
- No oversized logo, marketing headline, glass effect, background illustration, or decorative animation in the workbench.

### Current baseline

Source inspection of `src/App.svelte` and `src/styles.css` shows a 44px header, 240px explorer, 36px tab strip, 28px breadcrumb, and 24px status bar. The empty editor centers a large monogram, headline, one action, and three shortcuts. The current accent is periwinkle, the editor imports One Dark, and the terminal specifies only background, foreground, and cursor colors. These independent theme sources all need attention.

Reusable surfaces already exist in `Tree.svelte`, `BottomPanel.svelte`, `Palette.svelte`, `Settings.svelte`, and `Modal.svelte`. Build on those boundaries. This is a source-based baseline, not a completed visual audit.

### Users and priorities

- A developer opening a project needs an obvious starting action and immediately visible project context.
- A developer editing and running files needs fast navigation, readable code, and output adjacent to the work.
- A keyboard user needs visible focus, predictable navigation, and reliable focus restoration.
- A user with low vision or a small window needs scalable text and collapsible panels without losing commands.

## 2. Color

### Shared dark tokens

| Token | Value | Purpose |
| --- | --- | --- |
| `--canvas` | `#121212` | Editor and main workspace |
| `--sidebar` | `#1A1A1A` | Explorer, toolbar, inactive tabs |
| `--raised` | `#242424` | Dialogs, menus, inputs on canvas |
| `--hover` | `#303030` | Hovered controls and rows |
| `--pressed` | `#3A3A3A` | Pressed neutral controls |
| `--border` | `#3D3D3D` | Panel dividers and quiet outlines |
| `--control-border` | `#808080` | Input boundaries where needed for recognition |
| `--text` | `#EEEEEE` | Primary text |
| `--muted` | `#BDBDBD` | Paths, descriptions, secondary labels |
| `--text-subtle` | `#A0A0A0` | Line numbers and low-priority metadata |
| `--accent` | `#D0D0D0` | Neutral primary actions, active indicators, links, focus |
| `--accent-hover` | `#E0E0E0` | Hovered primary action |
| `--accent-pressed` | `#BDBDBD` | Pressed primary action |
| `--on-accent` | `#121212` | Text on filled light-grey buttons |
| `--selection` | `#3A3A3A` | Selected rows and editor selection |
| `--success` | `#86D4A0` | Successful completion |
| `--warning` | `#E8BD75` | Caution and unsaved marker |
| `--error` | `#F18D86` | Failures and destructive actions |
| `--overlay` | `#00000099` | Modal backdrop |
| `--shadow` | `#00000066` | Elevated surface shadow |

Use neutral surfaces for most pixels. Light grey indicates interaction or selection; amber and red communicate state. Never communicate state through color alone. A selected explorer row also has a 2px light-grey leading marker; an active tab has a 2px light-grey top rule and stronger text. Underline inline links so they remain distinguishable from ordinary text. Hovering a selected row retains its selection marker.

### Editor syntax

| Role | Color |
| --- | --- |
| Plain text, punctuation, variables | `--text` |
| Keywords and operators | `--accent` |
| Strings | `--success` |
| Numbers, constants, booleans | `--warning` |
| Functions and callable names | `--accent-hover` |
| Types and class names | `#D9D59A` |
| Comments | `--text-subtle` |
| Invalid syntax | `--error`, with an additional underline |

Replace or fully override the current One Dark highlighting. Theme gutters, active line, bracket matching, selection, search matches, tooltips, and find/replace controls as part of the same system. Use `--sidebar` for the active line and `--selection` for matches, with an amber outline on the current search match. Do not rely on a preset whose colors bypass this palette.

### Terminal palette

Specify all 16 ANSI slots, plus cursor and selection. ANSI slot names are protocol labels, not permission to show purple.

| ANSI slot | Normal | Bright |
| --- | --- | --- |
| Black | `#1A1A1A` | `#808080` |
| Red | `#F18D86` | `#FFB0A8` |
| Green | `#86D4A0` | `#AFE8BE` |
| Yellow | `#E8BD75` | `#F5D99E` |
| Blue | `#79BCEE` | `#ACD8F7` |
| Magenta, remapped to amber | `#D7A565` | `#EBC88F` |
| Cyan | `#55C7E8` | `#85DCF2` |
| White | `#D0D0D0` | `#EEEEEE` |

Terminal background uses `--canvas`, foreground uses `--text`, cursor uses `--accent`, and selection uses `--selection`.

The no-purple requirement governs colors Nicle owns. User-authored preview pages and explicit true-color terminal output can supply their own colors; preserve their content rather than recoloring it. Nicle's bundled examples and test fixtures must follow this palette.

Dark becomes the fresh-install default. Preserve an existing explicit light preference for compatibility; any retained light theme must also replace purple accents and syntax. Do not silently reset saved preferences as part of styling.

## 3. Typography

Use locally bundled IBM Plex Sans for interface text and IBM Plex Mono for code, terminal text, paths, and shortcut hints. Fall back to `system-ui, sans-serif` and `ui-monospace, monospace` respectively. The app must remain usable offline with fallback fonts.

| Role | Size / line height | Weight |
| --- | --- | --- |
| Welcome title | 24px / 32px | 600 |
| Dialog title | 18px / 24px | 600 |
| Body and form labels | 14px / 20px | 400 |
| Tree rows, tabs, buttons | 13px / 20px | 400; active 500 |
| Panel labels, metadata, shortcuts | 12px / 16px | 500 |
| Code and terminal default | 14px / 22px | 400 |

Use sentence case, normal tracking, and no tiny all-caps headings. Keep the existing editor font-size setting authoritative. Do not overwrite a saved size with the new default. Truncate paths in chrome only, expose their full value on focus or hover, and allow text to wrap in dialogs and empty states.

## 4. Spacing and layout

Spacing tokens: `--space-1: 4px`, `--space-2: 8px`, `--space-3: 12px`, `--space-4: 16px`, `--space-6: 24px`, `--space-8: 32px`, `--space-12: 48px`.

| Region | Target geometry |
| --- | --- |
| Workspace toolbar | 44px high |
| Explorer | 256px default, resizable from 200–360px |
| Panel headings | 36px high |
| Tree rows | 28px high, 16px indentation per level |
| Editor tabs | 36px high, 120–220px wide |
| Breadcrumb | 28px high |
| Status bar | 24px high |
| Bottom panel | 240px initial height when opened; adjustable |
| Icon buttons | At least 28 × 28px; icons 16px |
| Standard form controls | 36px high |
| Splitter | 1px visible divider, 8px pointer target |

The workbench fills the window. Do not place the entire IDE inside a centered max-width container. Panels scroll independently; the toolbar and status bar stay visible. Keep at least 240px of editor height when the bottom panel is open, collapsing secondary regions when the window cannot accommodate both.

### Workspace composition

```text
+------------------------------------------------------------------------+
| nicle.  workspace       [ Search files…       Ctrl P ]   Run  Tools      |
+------------------+-----------------------------------------------------+
| Explorer    + …  | file.ts  × | styles.css  •                           |
| v workspace     +-----------------------------------------------------+
|   v src          | src / file.ts                                       |
|     file.ts      +-----------------------------------------------------+
|     styles.css   |  1  Code editor                   | HTML preview*    |
|   README.md      |  2                                |                  |
|                  |  3                                |                  |
|                  +-----------------------------------------------------+
|                  | Terminal  Output                  Restart  Stop  ×  |
|                  | Actual shell or runner output                       |
+------------------+-----------------------------------------------------+
| Ready / process state                         Ln 1, Col 1   UTF-8  TS   |
+------------------------------------------------------------------------+
```

Illustrative filenames only; render real workspace data. The bottom panel appears on demand or from restored layout state. Preview appears only after a preview action. Do not launch a process to make the layout look occupied.

### Window behavior

- At 1280px and wider: expanded explorer; editor and optional preview side by side, with preview initially taking 40% of the editing region.
- At 900–1279px: explorer defaults to 220px; reduce toolbar labels before reducing editor space. Keep Search, Run when relevant, and an accessible overflow menu.
- Below 900px: explorer becomes a dismissible overlay. Preview and editor become switchable views if each cannot retain 360px of usable width.
- At 375–767px: one main view at a time, compact search trigger, stacked welcome sections, and dialogs inset 16px. Keep every command reachable through the command palette or overflow.
- At short heights: constrain dialogs to the viewport and scroll their content. Preserve access to actions and the bottom-panel hide control.
- Code and terminal may scroll horizontally; application chrome must not cause horizontal page scrolling.

## 5. Components and screen specifications

### Material UI component language

- Use a compact app bar, drawer-like explorer, contained and outlined buttons, outlined fields, tabs, menus, dialogs, and snackbar-style notifications. Share spacing, shape, typography, and state tokens across these primitives.
- Primary buttons use a solid `--accent` fill with `--on-accent` text, `--accent-hover` on hover, and `--accent-pressed` when pressed. Secondary buttons use a transparent fill and `--control-border`; tertiary actions use text or icon buttons. Keep labels in sentence case and avoid competing primary actions within a group.
- Neutral buttons and rows use solid `--hover` and `--pressed` state fills. Disabled controls use `--text-subtle` on their existing surface, omit elevation, and suppress hover feedback. Pending actions retain their label and show a progress indicator without changing width.
- Fields use a solid `--raised` fill, 1px `--control-border` outline, a persistent label, and helper or error text below. Focus adds the shared ring; invalid fields also show an error icon or message. Never use placeholder text as the only label.
- Settings toggles use a clear switch track and thumb; checked checkboxes show a visible checkmark. Active states use light grey with dark foreground details. Preserve native semantics and keyboard behavior.
- Menus and dialogs use the raised surface and neutral elevation. Separate dialog title, content, and trailing actions with consistent spacing; use dividers only when they clarify groups. Keep the workbench dense while allowing comfortable form spacing.

### Workspace toolbar

Use three groups: identity and explorer toggle; file search; contextual actions. Keep the workspace name readable. The search control shows “Search files…” and the platform shortcut. Group Run and Stop together; expose Preview for HTML. Move infrequent commands into overflow at constrained widths. Settings remains discoverable. Avoid adding an activity rail with mostly empty destinations.

### Explorer

Keep a compact heading with New file, New folder, and Refresh actions. Show the workspace root, folder chevrons, consistent file icons, indentation guides, and a full-width active row. Reuse the installed Lucide icons with a consistent stroke weight. Retain row action menus and make them visible on keyboard focus as well as hover.

When no folder is open, show “No folder open,” a short explanation, and “Open folder.” When a folder is empty, offer “New file” and “New folder.” Show loading and access errors at the affected location with a retry action. Do not add fake file counts or Git badges.

### Welcome and no-editor states

Replace “Space to code.” and the oversized monogram with a left-aligned start surface, maximum width 880px, positioned 48px below the editor area's top edge with 32px side padding. Use two unequal columns: actions on the left, shortcut reference on the right. Reduce padding to 16px in narrow windows.

**No workspace:** title “Start a workspace,” description “Open a folder to browse files, edit code, and run commands.” Primary action “Open folder”; secondary action “Command palette.” The adjacent “Keyboard shortcuts” list shows Open file, Command palette, Save, and Toggle terminal with platform-correct keys. Mark workspace-dependent actions unavailable with a clear reason.

**Workspace open, no file selected:** title is the actual workspace name; show the path below it. Offer “Open file,” “New file,” and “Open terminal.” Keep the populated explorer visible. Add a compact section explaining “Run a supported file” and “Preview an HTML file,” with actions enabled only when applicable.

Use aligned rows and section dividers instead of a dashboard card grid. Do not show recent projects until persistence exists; it is optional future scope, not a placeholder to populate.

### Tabs, breadcrumb, and editor

Use a canvas-colored active tab, quiet inactive tabs, a light-grey top indicator, and an amber dirty dot with an accessible “Unsaved changes” label. Keep close controls reachable without accidental tab activation. Scroll overflowing tabs and ensure the active tab is visible.

Breadcrumbs use workspace-relative paths. Keep full absolute paths available without dominating the editor. The editor includes readable line numbers, a subtle active line, visible cursor and selection, and integrated find/replace styling. Preserve per-tab undo and selection. Do not add a decorative minimap or unsupported diagnostics.

### Terminal and output

Keep Terminal and Output as adjacent tabs in one bottom panel, with shared heading geometry. Show real running or stopped state with a label and icon. Keep Stop, Restart, Clear output, Hide panel, and End terminal distinguishable. Hiding must preserve the shell; ending the process remains an explicit action.

An unstarted terminal shows “No terminal session” and “Start terminal.” Output with no run shows “Run a file to see its output here.” Running output remains monospaced and selectable. Only show exit status or timing if available from actual process state. Preserve panel size during the session; restoring it across launches requires explicit layout persistence work.

### HTML preview

Frame the preview with a dark toolbar containing the real filename, Refresh, and Close. Keep page content unmodified. Support a resizable divider on wide windows and a dedicated preview view on narrow windows. Preserve existing refresh and server-lifecycle behavior. Distinguish a loading page, a preview error with retry, and a running preview using actual available events.

### Quick Open and command palette

Use a dialog up to 640px wide, placed near the upper third of the workbench. Put search first, results second, and keyboard instructions in a quiet footer. File results show filename and secondary relative path. Command results show action and shortcut. Use selection plus a leading indicator for the current result.

Cover loading, no matches, and search errors with different messages. Preserve keyboard navigation, Enter activation, Escape dismissal, and focus restoration. A retry action must retry the search rather than clear the error only.

### Settings, dialogs, and notifications

Group Settings into Appearance and Editor sections within the existing dialog. Show theme, font size, indentation, and word wrap with clear labels and enough spacing to scan. State that preferences save on this device.

Keep standard dialogs up to 480px wide. Use specific submit labels such as “Open folder,” “Create file,” “Rename,” and “Delete permanently” instead of “Confirm.” Preserve Save / Discard / Cancel for dirty files. Destructive actions use red and name the affected item.

Notifications use a raised surface, severity icon, readable message, and dismiss control. Important errors persist until dismissed or resolved. Long paths wrap. Do not obscure terminal controls or steal focus.

### Shared primitive contract

| Primitive | Required states |
| --- | --- |
| Button / icon button | Default, hover, pressed, focus-visible, disabled, pending |
| Tree / palette row | Default, hover, selected, focused, unavailable |
| Tab | Inactive, active, hover, focused, dirty |
| Input / select | Default, hover, focused, disabled, invalid |
| Splitter | Default, hover, keyboard-focused, dragging |
| Dialog | Open, pending action, validation error, dismissal and focus return |
| Tool panel | Hidden, empty, loading, populated, failed |
| Notification | Informational, successful, warning, error |

Extract shared presentation where it repeats; retain feature-specific behavior in existing components. Verify primitives and their states in an isolated showcase before composing redesigned screens during implementation.

## 6. Interaction and motion

- Existing shortcuts remain authoritative: Ctrl/Command+P for Quick Open, Ctrl/Command+Shift+P for commands, Ctrl/Command+S for Save, and Ctrl/Command+backtick for terminal. Display platform-specific labels.
- Hover feedback is immediate. Dialog and menu entry may use 120ms opacity; overlay explorer entry may use 160ms transform and opacity. Use `cubic-bezier(0.2, 0, 0, 1)` for entry.
- Do not animate editor typing, terminal output, panel dimensions, or tab switching. Splitters track pointer movement directly.
- Reduced motion removes entry movement and fades. Loading messages remain understandable without animation.
- Splitters support keyboard resizing with arrow keys and expose orientation and current size. Explorer overlays close on Escape and restore focus to their trigger.
- Disabled actions explain their prerequisite through nearby text or an accessible description. Tooltips must not be the only source of essential instructions.

## 7. Depth, borders, and surfaces

Use square workbench regions with 1px shared dividers. Shape tokens: `--radius-control: 4px` for buttons and fields, `--radius-menu: 8px` for menus and popovers, and `--radius-dialog: 12px` for dialogs. Avoid rounding the explorer, editor, or terminal into floating cards.

Depth comes from the progression `--canvas` → `--sidebar` → `--raised`. Reserve `0 12px 32px var(--shadow)` for floating menus and dialogs. Use no accent glow, colored shadow, gradient, glass blur, or background texture. A stronger border belongs on interactive form controls; subtle dividers separate regions without competing with code.

Material elevation must use explicit solid background colors. Set application-owned surface `background-image` to `none`, including hover, selected, and elevated states. Material UI's dark Paper elevation uses a background-image gradient by default; if MUI is used in a future implementation, override both the background color and background image rather than inheriting that treatment. Reference: [Material UI Paper elevation](https://mui.com/material-ui/react-paper/#elevation), checked through Context7.

## 8. Accessibility, handoff, and acceptance

### Accessibility constraints

- Verify at least 4.5:1 contrast for normal text and 3:1 for meaningful control boundaries and focus indicators. Token selection alone is not verification.
- Use a 2px light-grey `--accent` focus ring with a 2px dark offset so it remains visible on selected controls and filled buttons. Never remove focus styling from editor, terminal, or dialog actions.
- Provide accessible names for every icon button, tab state announcements, labelled inputs, and programmatically associated errors.
- Support full keyboard operation, modal focus containment, Escape dismissal where safe, and focus restoration.
- Preserve readable content and reachable controls at 200% text scaling. Use a comfortable 44px target for coarse pointers while retaining compact desktop rows.
- Status changes should be announced without moving focus. Do not announce every terminal output line through a live region.
- Color is supplementary: dirty state, process state, errors, and selection also use text, shape, or icons.

### Implementation map

| Location | Redesign responsibility |
| --- | --- |
| `src/styles.css` | Shared tokens, typography, shell, component states, responsive rules |
| `src/App.svelte` | Toolbar, welcome states, tabs, breadcrumb, preview, status bar |
| `src/components/Tree.svelte` | Explorer rows, selection, actions, loading and empty states |
| `src/components/BottomPanel.svelte` | Terminal/output framing, resize behavior, process controls |
| `src/components/Palette.svelte` | Search/result hierarchy and keyboard states |
| `src/components/Settings.svelte` | Grouped, readable preference controls |
| `src/components/Modal.svelte` | Dialog structure, focus behavior, common actions |
| `src/editor/documents.ts` | Unified editor theme and no-purple syntax |
| `src/terminal/Terminal.svelte` | Full terminal palette and font alignment |

Implement in this order: shared tokens and primitive showcase; shell and welcome states; explorer and editor; terminal and preview; palette, settings, and dialogs; responsive and accessibility verification. Consult current library documentation when implementing API changes.

New layout behavior includes explorer resizing, adaptive preview switching, and any layout persistence. Treat these as implementation work, not capabilities already present. Recent projects, Git panels, extensions, AI chat, debugging, and project diagnostics are outside this redesign's required scope.

### Acceptance checklist for the implemented redesign

- [ ] Fresh launch is dark, with a useful welcome screen and no oversized centered logo.
- [ ] Open workspace and empty folder states offer relevant, working actions.
- [ ] The explorer, tabs, editor, preview frame, terminal, menus, settings, and dialogs follow the same tokens.
- [ ] Application chrome uses neutral dark and grey surfaces with Material-style buttons, fields, tabs, dialogs, and consistent state treatments; no cyan or other chromatic brand accent remains in chrome.
- [ ] No application-owned gradient appears in any default, hover, selected, focused, or elevated state; inspect background images and pseudo-elements as well as visible fills.
- [ ] Application-owned colors contain no purple-family hues; inspect computed styles, syntax samples, terminal ANSI slots, selections, and focus states.
- [ ] A real folder can be opened; a file created, edited, saved, renamed, and deleted with correct confirmation.
- [ ] Dirty-file close and workspace switch preserve Save / Discard / Cancel behavior.
- [ ] Quick Open, commands, find/replace, and existing shortcuts work with keyboard-only navigation.
- [ ] A supported file runs; Output shows real output; Stop and Restart work.
- [ ] A real terminal starts, accepts input, survives hiding, and ends only through its process action.
- [ ] HTML preview opens, refreshes saved changes, and closes cleanly.
- [ ] Long filenames, many tabs, errors, empty results, and large output remain readable.
- [ ] Verify 1440×900, 1280×800, 900×700, 768×700, and 375×812 layouts, plus 200% text scaling and reduced motion.
- [ ] Capture actual implementation screenshots and interaction evidence; browser-only rendering cannot verify native filesystem or process workflows.

### Evidence and accepted debt

This deliverable specifies the redesign and its verification criteria. It does not claim the redesign has been implemented, contrast-tested, or visually validated. No accessibility debt is accepted. Native desktop behavior and cross-platform typography remain verification obligations for implementation.
