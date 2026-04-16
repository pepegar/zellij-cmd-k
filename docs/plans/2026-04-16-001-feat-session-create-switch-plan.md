---
title: Add session create and switch support to zellij-cmd-k
type: feat
status: completed
date: 2026-04-16
---

# Add session create and switch support to zellij-cmd-k

## Overview

The cmd-k plugin already lists existing zellij sessions and lets the user switch to one (`Command::SwitchSession`). It does not expose a way to **create** a new session from the palette. This plan adds two create-session commands — one for an auto-named session (zero-prompt) and one for a user-named session (text input prompt) — and reuses the existing switching flow. Switching is already implemented; this plan only addresses the gaps around creation and the UX surface that binds them together.

## Problem Frame

Users live inside zellij sessions and frequently want to create a new one (e.g., for a different project, a scratch context, or to reorganize work) without leaving the keyboard or the palette. Today they must dismiss the palette, enter session mode via Zellij's own keybinds (`Ctrl+o w`), and go through the session manager UI. Putting session creation behind `Ctrl+k` makes it the same two-step flow as every other palette action: open palette → pick command.

## Requirements Trace

- R1. Offer a palette command that creates a brand-new zellij session with no user-provided name and switches to it immediately.
- R2. Offer a palette command that prompts the user for a session name inside the palette, then creates and switches to a session with that name.
- R3. Preserve existing "Go to session: X" behavior (no regression).
- R4. When the user is in the new-session-name prompt, `Esc` returns to the palette's browse mode (not dismiss); `Enter` confirms; `Backspace` edits.
- R5. Attempting to create a session whose name matches an existing live session should do the intuitive thing — switch to that existing session (`switch_session(Some(name))` already behaves this way per zellij-tile 0.43.1).
- R6. Empty / whitespace-only names from the prompt must not be sent to zellij; they should either no-op or fall back to the auto-named path.

## Scope Boundaries

**In scope:**
- Two new palette commands: `NewSession` (auto-named) and `NewSessionPrompt` (enters a name-input mode).
- A small state machine extension so the palette can render a name prompt instead of the command list.
- Minimal validation of the entered session name (trim, reject empty).

**Out of scope / non-goals:**
- Layouts and working-directory selection when creating a session (`switch_session_with_layout`, `switch_session_with_cwd`). These are deliberately deferred to keep this plan focused.
- Killing or renaming sessions (`kill_sessions`, `rename_session`, `delete_dead_session`). Out of scope.
- Resurrectable / dead session management.
- Remembering recently created session names or offering completions based on them.

## Context & Research

### Relevant Code and Patterns

- `src/commands.rs` — `Command` enum, `label()`, `category()`, `build_commands`, `filter_commands`. New variants are added here and in `build_commands`.
- `src/input.rs` — `handle_key` and `execute_command`. New variants need key-handling branches and command execution. The text-input prompt mode lives primarily here.
- `src/state.rs` — `State` struct. The `show_keybindings: bool` flag is the existing precedent for "render a non-default view". This plan generalizes that idea by introducing an explicit `view: View` enum so multiple non-default views don't keep creeping in as booleans. The memory note mentions a `Mode` enum (Browse | TextInput); this plan materializes that note in code.
- `src/render.rs` — `render()` dispatches to `render_keybindings` when a flag is set. A new `render_new_session_prompt` follows the same shape.
- `src/main.rs` — `register_plugin!`, `ZellijPlugin` impl. No changes needed beyond the existing permissions (`ChangeApplicationState` already covers session switching).

### Institutional Learnings

From `memory/MEMORY.md`:
- `switch_tab_to()` is 1-indexed, `TabInfo.position` is 0-indexed — not relevant here but confirms the palette treats zellij APIs as-is.
- `hide_self()` is used to dismiss the plugin keeping it loaded for instant re-open — the create-then-switch flow uses this path via the existing `close_self`.
- The plugin uses `NestedListItem` for theme-aware rendering — the command list already follows this; the new prompt view uses `print_text_with_coordinates` (matching the existing search input row at row 0).

### External References

- `zellij-tile 0.43.1` shim API (local source inspected at `~/.cargo/registry/src/index.crates.io-*/zellij-tile-0.43.1/src/shim.rs`):
  - `pub fn switch_session(name: Option<&str>)` — "Switch to a session with the given name, create one if no name is given". Passing `None` creates a session with an auto-generated name and switches to it. Passing `Some(name)` switches to that session, creating it if it doesn't exist. This single function covers both R1 and R2.
  - `pub fn switch_session_with_cwd` / `switch_session_with_layout` / `switch_session_with_focus` exist but are deferred (see Scope Boundaries).

## Key Technical Decisions

- **Use `switch_session(None)` for auto-named, `switch_session(Some(name))` for named.** Rationale: one API call covers both creation and switching, and matches how zellij's native session manager behaves. Matches R1, R2, R5.
- **Introduce `View` enum instead of adding more booleans to `State`.** Rationale: `show_keybindings: bool` was fine for one side-view, but adding a name-input prompt as another bool creates invalid states (both true simultaneously) and forces every call site to check both. An enum (`View::List | View::Keybindings | View::NewSessionPrompt { input: String }`) makes the state machine explicit, keeps invariants local, and folds `show_keybindings` and `keybindings_scroll` into variant-scoped data where appropriate.
- **Name-input prompt lives in the same palette pane, not a separate floating window.** Rationale: the plugin is already floating; nesting another floating pane would require separate plumbing. Reusing the existing render surface keeps the change small and visually consistent with the search input row.
- **Trim and reject empty names from the prompt.** Rationale: `switch_session(Some(""))` behavior is undefined / unwanted; trimming and falling back to auto-named or no-op is safer. Chosen behavior: on Enter with empty/whitespace input, fall back to `switch_session(None)` (i.e., "if you don't care, we don't care either"). This satisfies R6 without a dead-end UX.
- **Keep the `NewSession` quick command even though `NewSessionPrompt` could cover it.** Rationale: the fastest path ("new session, don't ask me anything") deserves to be a single-Enter command. Forcing every create through the prompt would add a useless empty-input step 90% of the time.

## Open Questions

### Resolved During Planning

- **What API does zellij-tile expose for creating a session?** — `switch_session` itself creates sessions; no separate `create_session` call. Resolved via zellij-tile shim inspection.
- **Should we support cwd/layout at creation time?** — Deferred to a future plan. Keeps this one bounded.
- **Do we need new permissions?** — No. `ChangeApplicationState` already covers session switching; creation goes through the same `SwitchSession` plugin command.
- **Does `SwitchSession` auto-create if the name is new?** — Yes, confirmed in the shim comment ("Switch to a session with the given name, create one if no name is given"). When a name is provided that doesn't exist, zellij creates it.

### Deferred to Implementation

- Exact placement of the new commands in the list (top? grouped with existing session command?) — decide based on rendering outcome; `build_commands` ordering is easy to tweak.
- Whether to show a small hint in the prompt view like `"Empty = auto-named"` — decide while implementing the render.

## High-Level Technical Design

> *This illustrates the intended approach and is directional guidance for review, not implementation specification. The implementing agent should treat it as context, not code to reproduce.*

Palette view state machine:

```
┌─────────────────┐   ShowKeybindings cmd   ┌──────────────────┐
│   View::List    │ ──────────────────────► │ View::Keybindings│
│ (search + list) │ ◄────────────────────── │                  │
└─────────────────┘         Esc             └──────────────────┘
        │  ▲
        │  │ Esc
        │  │
 NewSessionPrompt cmd
        │
        ▼                                     Enter (non-empty)
┌───────────────────────────────┐  ─────────────────────────► switch_session(Some(trimmed))
│ View::NewSessionPrompt        │                              hide_self()
│  input: String                │
└───────────────────────────────┘  ─────────────────────────► switch_session(None)
                                    Enter (empty)              hide_self()

NewSession cmd (from List view):
  switch_session(None) → hide_self()   // bypasses the prompt entirely
```

Key operations at each edge:
- `NewSession` command: executed directly from `View::List`; calls `switch_session(None)` and closes the plugin via `close_self`.
- `NewSessionPrompt` command: transitions `View::List → View::NewSessionPrompt { input: "" }`; does **not** close the plugin.
- `Esc` in `View::NewSessionPrompt`: transitions back to `View::List` (keeps palette open, preserves search term).
- `Enter` in `View::NewSessionPrompt`: trim input; empty → `switch_session(None)`; non-empty → `switch_session(Some(trimmed))`; then `close_self`.

## Implementation Units

- [x] **Unit 1: Introduce `View` enum and collapse existing view flags**

**Goal:** Replace `show_keybindings: bool` + `keybindings_scroll: usize` with a `View` enum so new prompt views don't introduce mutually-exclusive booleans.

**Requirements:** R4 (prerequisite for clean prompt-mode key handling).

**Dependencies:** None.

**Files:**
- Modify: `src/state.rs`
- Modify: `src/main.rs` (only if field references need updating)
- Modify: `src/input.rs` (replace `state.show_keybindings` checks with `matches!(state.view, View::Keybindings { .. })` etc.)
- Modify: `src/render.rs` (dispatch on `state.view` instead of the bool)

**Approach:**
- Add `pub enum View { List, Keybindings { scroll: usize }, NewSessionPrompt { input: String } }` in `state.rs`.
- Replace `show_keybindings` and `keybindings_scroll` fields with a single `view: View` field (default `View::List`).
- Update call sites: `input::handle_key` branches that read/write `show_keybindings` or `keybindings_scroll` now read/write through `state.view` pattern matches.
- Keep the `Command::ShowKeybindings` variant; its execution now sets `state.view = View::Keybindings { scroll: 0 }`.

**Patterns to follow:**
- The existing `show_keybindings` flag handling in `input::handle_key` (Esc, Up, Down) and `render::render` (dispatch to `render_keybindings`).

**Test scenarios:**
<!-- This unit is a pure state refactor — behavior must be equivalent, not new. -->
- Integration (manual run): Ctrl+k opens palette in list view (baseline).
- Integration (manual run): selecting `ShowKeybindings` from the list transitions to keybindings view; `Esc` returns to list view; search term is preserved.
- Integration (manual run): Up/Down scrolls the keybindings list as before.
- Test expectation: no new unit tests — refactor is covered by existing behavior and the manual runs above. (The crate has no `#[cfg(test)]` modules today; adding a test harness is out of scope.)

**Verification:**
- `cargo build --target wasm32-wasip1 --release` succeeds.
- All existing palette actions (switch tab, close tab, switch session, enter scroll/search mode, show keybindings, Esc to dismiss) work identically to before.
- No references to `show_keybindings` or `keybindings_scroll` remain in the codebase.

- [x] **Unit 2: Add `Command::NewSession` (auto-named create)**

**Goal:** One-Enter command that creates a fresh zellij session with an auto-generated name and switches to it.

**Requirements:** R1, R5.

**Dependencies:** Unit 1.

**Files:**
- Modify: `src/commands.rs` (add variant, label, category, include in `build_commands`)
- Modify: `src/input.rs` (handle new variant in `execute_command`)

**Approach:**
- Add `Command::NewSession` variant (no fields).
- `label()`: `"New session (auto-named)"`.
- `category()`: `"Session"` (groups with existing `SwitchSession`).
- In `build_commands`, push `Command::NewSession` alongside the static commands (e.g., right before or after `EnterScrollMode`) so it's always fuzzy-searchable.
- In `execute_command`, call `switch_session(None)` then `close_self(state)`.

**Patterns to follow:**
- `Command::SwitchSession` handling in `execute_command` — the closest analogue. Mirror its `close_self` call.

**Test scenarios:**
- Happy path: Open palette, type `new`, see `[Session] New session (auto-named)` ranked at or near the top, press Enter → zellij opens a new session and the palette closes.
- Happy path: Empty query, the command appears in the default list (verify it's present).
- Edge case: Pressing Enter on `NewSession` from any palette state (as long as the list view is active) creates the session without attempting to restore `origin_tab_position` (because we `close_self`, not `dismiss`).

**Verification:**
- Running the plugin, selecting the command, and observing a new session in the zellij session list.
- `Ctrl+k` reopens the palette inside the new session; the new session is marked `is_current_session` and therefore filtered out of the `SwitchSession` list (existing logic in `build_commands` handles this).

- [x] **Unit 3: Add `Command::NewSessionPrompt` and the prompt view**

**Goal:** From the list, users can pick "New session (named)..." to enter a text-input prompt, type a name, and create+switch to that session.

**Requirements:** R2, R4, R5, R6.

**Dependencies:** Unit 1 (needs `View::NewSessionPrompt`), Unit 2 (for consistency and because the two should be built and reviewed together).

**Files:**
- Modify: `src/commands.rs` (add `NewSessionPrompt` variant, label, category, include in `build_commands`)
- Modify: `src/input.rs` (execute transitions to prompt view; handle keys while in prompt view)
- Modify: `src/render.rs` (add `render_new_session_prompt` dispatched from `render()`)

**Approach:**
- `Command::NewSessionPrompt`:
  - `label()`: `"New session (named)..."`.
  - `category()`: `"Session"`.
  - `execute_command`: set `state.view = View::NewSessionPrompt { input: String::new() }`. Do **not** call `close_self` — we stay inside the palette.
- Key handling (added to `handle_key` before the existing List-view branches, gated on `matches!(state.view, View::NewSessionPrompt { .. })`):
  - `Esc` → `state.view = View::List` (does not dismiss, does not clear search_term).
  - `Enter` → take the input, trim; if empty, `switch_session(None)`, else `switch_session(Some(&trimmed))`; then `close_self(state)` (which also resets `state.view` to `View::List` when clearing).
  - `Backspace` → pop last char of `input`.
  - `Char(c)` (no Ctrl/Alt modifiers, not `\n`) → push to `input`.
  - Ctrl/Alt-modified keys and other bare keys in prompt view → fall through / no-op.
- Render (`render_new_session_prompt(state, rows, cols)`):
  - Row 0: title-ish line `" 🆕 New session name:"`.
  - Row 1: `" > <input>|"` (cursor bar, same style as search input at row 0 of the list view).
  - Row 2: hint `"   (Empty = auto-named)"` or similar (optional, finalize during impl).
  - Last row: `" Enter create | Esc back"`.
- `render::render` dispatches: `View::List` → current behavior; `View::Keybindings { .. }` → `render_keybindings`; `View::NewSessionPrompt { .. }` → `render_new_session_prompt`.
- `close_self` should reset `state.view = View::List` in addition to clearing `search_term` and `selected_index`, so the palette reopens in list view.

**Patterns to follow:**
- Existing search-input rendering at row 0 of `render::render` (the `format!(" > {}|", ...)` style).
- Existing keybindings-view render dispatch.

**Test scenarios:**
- Happy path: Open palette, type `named`, select `New session (named)...`, press Enter → view switches to prompt. Type `scratch`, press Enter → new session `scratch` is created and switched to.
- Happy path: In prompt view, type nothing, press Enter → falls back to auto-named session (R6).
- Edge case: In prompt view, type `  spaces  `, Enter → trims to `spaces`, creates session `spaces`.
- Edge case: In prompt view, type a name that matches an existing live session → `switch_session(Some(name))` switches to it (documented zellij behavior; R5).
- Edge case: Backspace on empty input is a no-op (no panic).
- Edge case: Press `Esc` in prompt view → returns to list view. The previous `search_term` is preserved (so the user can continue filtering).
- Edge case: Ctrl+k / Ctrl+c / other modified keys while in prompt view do not get appended as characters.
- Integration: After successful creation, reopening the palette inside the new session shows the new session as the current one (not listed under `SwitchSession`).

**Verification:**
- `cargo build --target wasm32-wasip1 --release` succeeds.
- End-to-end manual run of each test scenario above inside a zellij instance with the built `.wasm` loaded.
- No panic, no stuck views, `Esc` always provides a reversible exit from the prompt.

- [x] **Unit 4: Update keybindings hint and (optionally) README/docs**

**Goal:** Reflect the new commands in user-facing hints and any documentation.

**Requirements:** R1, R2 (discoverability).

**Dependencies:** Units 2, 3.

**Files:**
- Modify: `src/render.rs` — the list-view hint bar already reads `" Enter select | Esc close | Up/Down navigate | ? keybindings"`. No change needed unless the hint line grows stale; verify and tweak only if necessary.
- Modify: `README.md` if one exists and documents available commands. (None is present at plan time; create one only if the user asks. Do not create otherwise.)

**Approach:**
- Scan existing user-facing strings; if anything enumerates available commands, update it.
- Confirm the help-view rendering (`render_keybindings`) is unaffected — it reflects zellij's own keybindings, not plugin commands, so no change.

**Patterns to follow:**
- Existing hint-bar text conventions.

**Test scenarios:**
- Test expectation: none — this unit has no behavioral change beyond documentation touch-ups. If no strings need editing, the unit collapses to a no-op and can be closed without a commit.

**Verification:**
- Visual inspection of hint bars; no stale references remain.

## System-Wide Impact

- **Interaction graph:** The existing `Event::SessionUpdate` subscription continues to drive `state.sessions`. Creating a session via `switch_session` triggers zellij to switch the client; the plugin instance in the **old** session is hidden (`hide_self` / `close_self`) just before the switch, which matches the pattern used by `SwitchSession` today. When the client lands in the new session, a freshly-loaded or already-loaded plugin instance there handles its own lifecycle.
- **Error propagation:** `switch_session` does not return a Result — it's a one-way message to zellij. Failure modes (session creation failure, name collision with a dead session, etc.) are handled by zellij itself, not by the plugin. No additional error plumbing is needed in the plugin.
- **State lifecycle risks:** The `View` enum refactor must preserve the invariant that `Esc` is always a safe, reversible exit. The prompt view stores its own input string; resetting `state.view = View::List` on `close_self` prevents the next palette open from landing in a stale prompt with old input.
- **API surface parity:** No external API changes. The plugin's `register_plugin!(State)` signature is unchanged. Zellij config keybinding (`Ctrl k` → `LaunchOrFocusPlugin`) is unchanged.
- **Integration coverage:** The create-then-switch flow crosses the plugin boundary (plugin calls `switch_session`, zellij handles the switch). Unit tests cannot verify this end-to-end; manual runs in zellij are the required integration proof.
- **Unchanged invariants:** `switch_tab_to` is still 1-indexed with 0-indexed positions; `hide_self` still keeps the plugin loaded for instant re-open; the floating-pane keybinding config is untouched; all previous palette commands retain their current labels, categories, and execution behavior.

## Risks & Dependencies

| Risk | Mitigation |
|------|------------|
| `switch_session(Some(""))` has undefined/undesired behavior | Trim + fall back to `switch_session(None)` on empty input (R6). |
| `View` enum refactor regresses keybindings view | Unit 1 is a pure refactor with the manual-run verification list; land it before Units 2/3 so regressions surface without feature noise. |
| Creating a session with a name that collides with a live session quietly switches instead of creating | This is the documented zellij-tile behavior and is the intuitive outcome (R5). Document it once in the prompt hint if needed ("existing names switch instead of create" is a nice-to-have hint; optional). |
| Prompt view traps the user if `Esc` handling breaks | `Esc` handling is a dedicated early branch in `handle_key`; covered by test scenarios in Unit 3. |
| Auto-named session has an unpredictable name the user can't find | Zellij auto-names are discoverable via the next palette open's session list; not a blocker. |

## Documentation / Operational Notes

- No rollout, monitoring, or migration concerns — this is a local WASM plugin rebuilt and reloaded by the user.
- The existing build command `cargo build --target wasm32-wasip1 --release` inside `nix develop` is the only artifact-producing step. The user's Zellij `config.kdl` keybinding path does not change.
- If the user maintains a README (none at plan time), the "Commands" section would gain two lines: `New session (auto-named)` and `New session (named)...`.

## Sources & References

- zellij-tile shim: `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/zellij-tile-0.43.1/src/shim.rs` lines 820–876 (`switch_session` and variants).
- Current plugin source:
  - `src/commands.rs` (Command enum, build/filter logic)
  - `src/input.rs` (key handling, command execution)
  - `src/render.rs` (list + keybindings rendering)
  - `src/state.rs` (State struct)
  - `src/main.rs` (ZellijPlugin impl, event subscriptions)
- Project memory: `memory/MEMORY.md` — Build notes, keybinding config, architecture decisions.
