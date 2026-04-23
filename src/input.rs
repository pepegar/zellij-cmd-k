use zellij_tile::prelude::*;

use crate::commands::Command;
use crate::state::{State, View};

pub fn handle_key(state: &mut State, key: KeyWithModifier) -> bool {
    // While editing a text-prompt view, route *all* keys to the prompt
    // handler so shortcuts like '?' don't leak through. Esc returns to List.
    if matches!(state.view, View::NewSessionPrompt { .. }) {
        return handle_key_new_session_prompt(state, key);
    }
    if matches!(state.view, View::RenameSessionPrompt { .. }) {
        return handle_key_rename_session_prompt(state, key);
    }

    // '?' shortcut when search is empty → show keybindings
    if let BareKey::Char('?') = key.bare_key {
        if state.search_term.is_empty()
            && !key.has_modifiers(&[KeyModifier::Ctrl])
            && !key.has_modifiers(&[KeyModifier::Alt])
            && matches!(state.view, View::List)
        {
            execute_command(state, &Command::ShowKeybindings);
            return true;
        }
    }

    match key.bare_key {
        BareKey::Esc => {
            if matches!(state.view, View::Keybindings { .. }) {
                state.view = View::List;
            } else {
                dismiss(state);
            }
            true
        }
        BareKey::Enter => {
            if matches!(state.view, View::Keybindings { .. }) {
                return true; // no-op in keybindings view
            }
            if let Some(scored) = state.filtered_commands.get(state.selected_index) {
                let cmd = scored.command.clone();
                execute_command(state, &cmd);
            }
            true
        }
        BareKey::Up => {
            if let View::Keybindings { scroll } = &mut state.view {
                *scroll = scroll.saturating_sub(1);
            } else {
                let len = state.filtered_commands.len();
                if len > 0 {
                    state.selected_index = if state.selected_index == 0 {
                        len - 1
                    } else {
                        state.selected_index - 1
                    };
                }
            }
            true
        }
        BareKey::Down => {
            if let View::Keybindings { scroll } = &mut state.view {
                *scroll += 1;
            } else {
                let len = state.filtered_commands.len();
                if len > 0 {
                    state.selected_index = (state.selected_index + 1) % len;
                }
            }
            true
        }
        BareKey::Backspace => {
            state.search_term.pop();
            state.selected_index = 0;
            state.refilter();
            true
        }
        BareKey::Char(c) => {
            if c == '\n' {
                // Enter can arrive as Char('\n') in some terminals
                if !matches!(state.view, View::Keybindings { .. }) {
                    if let Some(scored) = state.filtered_commands.get(state.selected_index) {
                        let cmd = scored.command.clone();
                        execute_command(state, &cmd);
                    }
                }
                true
            } else if !key.has_modifiers(&[KeyModifier::Ctrl])
                && !key.has_modifiers(&[KeyModifier::Alt])
            {
                state.search_term.push(c);
                state.selected_index = 0;
                state.refilter();
                true
            } else {
                false
            }
        }
        _ => false,
    }
}

fn handle_key_new_session_prompt(state: &mut State, key: KeyWithModifier) -> bool {
    match key.bare_key {
        BareKey::Esc => {
            // Back to the list view; preserve search_term so the user can resume filtering.
            state.view = View::List;
            true
        }
        BareKey::Enter => {
            submit_new_session_prompt(state);
            true
        }
        BareKey::Backspace => {
            if let View::NewSessionPrompt { input } = &mut state.view {
                input.pop();
            }
            true
        }
        BareKey::Char(c) => {
            if key.has_modifiers(&[KeyModifier::Ctrl]) || key.has_modifiers(&[KeyModifier::Alt]) {
                false
            } else if c == '\n' {
                submit_new_session_prompt(state);
                true
            } else {
                if let View::NewSessionPrompt { input } = &mut state.view {
                    input.push(c);
                }
                true
            }
        }
        _ => false,
    }
}

fn handle_key_rename_session_prompt(state: &mut State, key: KeyWithModifier) -> bool {
    match key.bare_key {
        BareKey::Esc => {
            state.view = View::List;
            true
        }
        BareKey::Enter => {
            submit_rename_session_prompt(state);
            true
        }
        BareKey::Backspace => {
            if let View::RenameSessionPrompt { input } = &mut state.view {
                input.pop();
            }
            true
        }
        BareKey::Char(c) => {
            if key.has_modifiers(&[KeyModifier::Ctrl]) || key.has_modifiers(&[KeyModifier::Alt]) {
                false
            } else if c == '\n' {
                submit_rename_session_prompt(state);
                true
            } else {
                if let View::RenameSessionPrompt { input } = &mut state.view {
                    input.push(c);
                }
                true
            }
        }
        _ => false,
    }
}

fn submit_rename_session_prompt(state: &mut State) {
    let View::RenameSessionPrompt { input } = &state.view else {
        return;
    };
    let name = input.trim().to_string();
    if !name.is_empty() {
        rename_session(&name);
    }
    close_self(state);
}

fn submit_new_session_prompt(state: &mut State) {
    // Take an owned trimmed copy to end the immutable borrow on state.view
    // before the switch_session / close_self mutation below.
    let View::NewSessionPrompt { input } = &state.view else {
        return;
    };
    let name = input.trim().to_string();
    if name.is_empty() {
        switch_session(None);
    } else {
        switch_session(Some(&name));
    }
    close_self(state);
}

fn execute_command(state: &mut State, command: &Command) {
    match command {
        Command::SwitchToTab { position, .. } => {
            switch_tab_to(*position as u32 + 1);
            close_self(state);
        }
        Command::CloseTab { position, .. } => {
            close_tab_with_index(*position);
            close_self(state);
        }
        Command::SwitchSession { name } | Command::ResumeSession { name } => {
            switch_session(Some(name.as_str()));
            close_self(state);
        }
        Command::NewSession => {
            switch_session(None);
            close_self(state);
        }
        Command::NewSessionPrompt => {
            state.view = View::NewSessionPrompt {
                input: String::new(),
            };
        }
        Command::RenameSessionPrompt => {
            let current_name = state
                .sessions
                .iter()
                .find(|s| s.is_current_session)
                .map(|s| s.name.clone())
                .unwrap_or_default();
            state.view = View::RenameSessionPrompt {
                input: current_name,
            };
        }
        Command::EnterScrollMode => {
            close_self(state);
            switch_to_input_mode(&InputMode::Scroll);
        }
        Command::EnterSearchMode => {
            close_self(state);
            switch_to_input_mode(&InputMode::EnterSearch);
        }
        Command::ShowKeybindings => {
            state.view = View::Keybindings { scroll: 0 };
        }
    }
}

/// Hide the plugin and reset state without restoring the origin tab.
/// Used when a command has already navigated to the desired destination.
fn close_self(state: &mut State) {
    state.search_term.clear();
    state.selected_index = 0;
    state.view = View::List;
    hide_self();
}

/// Dismiss the plugin, restoring the tab that was focused before it was shown.
/// Used when cancelling (Esc) so that opening and closing the plugin is a noop.
fn dismiss(state: &mut State) {
    if let Some(pos) = state.origin_tab_position {
        switch_tab_to(pos as u32 + 1);
    }
    close_self(state);
}
