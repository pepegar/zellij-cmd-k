---
title: SessionUpdate event's resurrectable sessions were discarded, leaving no way to resume past sessions from the palette
date: 2026-04-16
category: docs/solutions/integration-issues
module: commands
problem_type: integration_issue
component: tooling
symptoms:
  - Palette shows no "switch session" option when only one live session exists
  - Past (stopped) sessions not listed, even though Zellij can resurrect them
  - Users can only create new sessions, not resume prior ones
root_cause: logic_error
resolution_type: code_fix
severity: medium
tags: [zellij, session-update, resurrectable-sessions, switch-session, palette]
---

# SessionUpdate event's resurrectable sessions were discarded, leaving no way to resume past sessions from the palette

## Problem

`zellij-cmd-k` built session commands only from the live sessions array delivered by `Event::SessionUpdate`. The second tuple element — the list of resurrectable (disconnected-but-on-disk) sessions — was destructured into `_durations` and thrown away, so users with only one live session had no session-switching entry in the palette and no way to resume prior sessions.

## Symptoms

- Palette shows no `Switch to session: …` rows when the user only has their current session.
- No entry to resume disconnected sessions that Zellij retains on disk.
- User report: "there's no way to switch to a different session?"

## What Didn't Work

- Checking `is_current_session` filtering — the filter was correct; it just had nothing to filter against because only live sessions were being stored.
- Considering a docs-only fix ("use the named prompt to create-or-switch") — technically works because `switch_session(Some(name))` creates-or-switches, but it's unhelpful UX: the user has to remember the exact name of a session they might have forgotten.

## Solution

`Event::SessionUpdate(Vec<SessionInfo>, Vec<(String, Duration)>)` carries both live sessions and resurrectable sessions. Capture both on the state, then emit a `ResumeSession { name }` command for each resurrectable that isn't also a live session.

State addition (`src/state.rs`):
```rust
use std::time::Duration;

pub struct State {
    // ...
    pub sessions: Vec<SessionInfo>,
    pub resurrectable_sessions: Vec<(String, Duration)>,
    // ...
}
```

Event wiring (`src/main.rs`):
```rust
Event::SessionUpdate(sessions, resurrectable) => {
    self.sessions = sessions;
    self.resurrectable_sessions = resurrectable;
    self.refilter();
    true
}
```

Command enum + builder (`src/commands.rs`):
```rust
pub enum Command {
    // ...
    SwitchSession { name: String },
    ResumeSession { name: String },
    // ...
}

pub fn build_commands(
    tabs: &[TabInfo],
    _pane_manifest: &Option<PaneManifest>,
    sessions: &[SessionInfo],
    resurrectable_sessions: &[(String, Duration)],
) -> Vec<Command> {
    // ... live sessions ...
    for (name, _duration) in resurrectable_sessions {
        if sessions.iter().any(|s| &s.name == name) {
            continue; // already live; don't show twice
        }
        commands.push(Command::ResumeSession { name: name.clone() });
    }
    commands
}
```

Executor (`src/input.rs`) — `switch_session(Some(name))` handles both switch-live and resurrect paths:
```rust
Command::SwitchSession { name } | Command::ResumeSession { name } => {
    switch_session(Some(name.as_str()));
    close_self(state);
}
```

Also relabeled the named-session prompt from `New session (named)...` to `New or switch to session (by name)...` since `switch_session(Some(name))` is create-or-switch, not create-only.

## Why This Works

`Event::SessionUpdate` in `zellij-utils` (see `data.rs`) is defined as:
```rust
SessionUpdate(
    Vec<SessionInfo>,
    Vec<(String, Duration)>, // resurrectable sessions
),
```

The second element is precisely the list we need. No extra permissions or subscriptions are required beyond `ReadApplicationState` — the event already includes resurrectable data.

`zellij_tile::shim::switch_session(Some(name))` is documented as "switch to a session with the given name, create one if no name is given" but in practice it's the universal entry point: it switches to a live session by name, resurrects a disk-retained session by name, or creates a new session if the name is unused. One command path handles all three cases (auto memory [claude]).

## Prevention

- When destructuring Zellij event payloads with `_unused` patterns, explicitly note in a comment why the field is being discarded. If no reason exists, capture it — Zellij event tuples exist to deliver data.
- Prefer named struct destructuring over positional tuple destructuring when reading Zellij events: the field names in `zellij-utils` are explicit (`resurrectable sessions`), and catching this at the pattern-match site would have flagged the discard.
- When adding any session-related palette entry, audit all three switch_session inputs: `None` (new auto-named), `Some(live_name)`, `Some(resurrectable_name)`. All go through the same `switch_session` call — the palette should surface all three paths.

## Related

- `docs/solutions/documentation-gaps/launch-or-focus-plugin-needs-move-to-focused-tab-2026-04-16.md` — sibling learning from the same debugging session, about floating-pane keybind setup.
- `zellij-tile` session API (auto memory [claude]): `switch_session(None)` creates auto-named; `Some(name)` creates-or-switches-or-resurrects. Related variants: `switch_session_with_cwd`, `switch_session_with_layout`, `switch_session_with_focus`.
