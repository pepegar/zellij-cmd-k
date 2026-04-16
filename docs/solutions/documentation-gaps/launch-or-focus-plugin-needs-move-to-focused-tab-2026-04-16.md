---
title: LaunchOrFocusPlugin keybind must set move_to_focused_tab true for floating plugins
date: 2026-04-16
category: docs/solutions/documentation-gaps
module: keybind-setup
problem_type: documentation_gap
component: tooling
severity: high
applies_when:
  - Binding a Zellij floating plugin to a global keybind (e.g. Ctrl+k)
  - Plugin is expected to appear over the currently focused tab
  - Plugin uses `hide_self()` to dismiss (staying loaded across opens)
tags: [zellij, keybinds, launchorfocusplugin, floating-pane, move-to-focused-tab]
---

# LaunchOrFocusPlugin keybind must set move_to_focused_tab true for floating plugins

## Context

When a Zellij plugin is bound with `LaunchOrFocusPlugin { floating true }` and the plugin uses `hide_self()` to dismiss (the pattern `zellij-cmd-k` uses so re-opens are instant), pressing the keybind from any tab other than the one the plugin was first launched in warps the client to that original tab before showing the floating pane.

This is Zellij's focus behavior, not a bug in the plugin: floating panes live inside a specific tab, and `LaunchOrFocusPlugin` focuses the pane wherever it was originally loaded. The fix lives entirely in the user's `config.kdl` keybind.

## Guidance

Always pair `floating true` with `move_to_focused_tab true` for palette-style plugins that are meant to appear over whatever tab the user is currently on:

```kdl
keybinds {
    shared_except "locked" {
        bind "Ctrl k" {
            LaunchOrFocusPlugin "file:~/.config/zellij/plugins/zellij-cmd-k.wasm" {
                floating true
                move_to_focused_tab true
            }
        }
    }
}
```

Document this requirement alongside the keybind snippet in the plugin's README / install instructions. A floating-pane palette without `move_to_focused_tab true` feels broken from the second press onward.

## Why This Matters

- **User experience:** Without the flag, every invocation after the first risks jumping the user to a tab they didn't ask to visit, destroying the "invisible overlay" feel of a command palette.
- **Not fixable plugin-side:** The plugin has no hook on re-focus to move itself, and `hide_self()` doesn't reset the owning tab. The only levers are on the keybind configuration.
- **Known pattern in the Zellij ecosystem:** The built-in `zellij:session-manager` keybind in Zellij's default config sets `move_to_focused_tab true` for the same reason.

## When to Apply

- Any plugin intended to be invoked from a global keybind while living in a floating pane
- Plugins that stay loaded across invocations (common when fast re-open matters)
- Keybind docs aimed at end users who may copy-paste without knowing Zellij's floating-pane semantics

## Examples

**Before (buggy UX):**
```kdl
bind "Ctrl k" {
    LaunchOrFocusPlugin "file:~/.config/zellij/plugins/zellij-cmd-k.wasm" {
        floating true
    }
}
```
Symptom: `Ctrl+k` from tab 5 jumps to tab 1 (where the plugin was first opened), then shows the floating palette on top of tab 1.

**After (correct UX):**
```kdl
bind "Ctrl k" {
    LaunchOrFocusPlugin "file:~/.config/zellij/plugins/zellij-cmd-k.wasm" {
        floating true
        move_to_focused_tab true
    }
}
```
Symptom: `Ctrl+k` from tab 5 shows the floating palette over tab 5. Dismissing returns focus to tab 5.

## Related

- `docs/solutions/integration-issues/resurrectable-sessions-missing-from-palette-2026-04-16.md` — sibling learning from the same debugging session, about surfacing resurrectable sessions.
- Zellij KDL: `LaunchOrFocusPlugin` accepts `floating`, `move_to_focused_tab`, and `should_open_in_place` children (see `zellij-utils` `src/kdl/mod.rs`).
