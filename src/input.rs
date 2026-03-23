use zellij_tile::prelude::*;

use crate::commands::Command;
use crate::state::State;

pub fn handle_key(state: &mut State, key: KeyWithModifier) -> bool {
    // '?' shortcut when search is empty → show keybindings
    if let BareKey::Char('?') = key.bare_key {
        if state.search_term.is_empty()
            && !key.has_modifiers(&[KeyModifier::Ctrl])
            && !key.has_modifiers(&[KeyModifier::Alt])
        {
            execute_command(state, &Command::ShowKeybindings);
            return true;
        }
    }

    match key.bare_key {
        BareKey::Esc => {
            if state.show_keybindings {
                state.show_keybindings = false;
                state.keybindings_scroll = 0;
            } else {
                dismiss(state);
            }
            true
        }
        BareKey::Enter => {
            if state.show_keybindings {
                return true; // no-op in keybindings view
            }
            if let Some(scored) = state.filtered_commands.get(state.selected_index) {
                let cmd = scored.command.clone();
                execute_command(state, &cmd);
            }
            true
        }
        BareKey::Up => {
            if state.show_keybindings {
                state.keybindings_scroll = state.keybindings_scroll.saturating_sub(1);
            } else {
                state.selected_index = state.selected_index.saturating_sub(1);
            }
            true
        }
        BareKey::Down => {
            if state.show_keybindings {
                state.keybindings_scroll += 1;
            } else if state.selected_index + 1 < state.filtered_commands.len() {
                state.selected_index += 1;
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
                if !state.show_keybindings {
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
        Command::SwitchSession { name } => {
            switch_session(Some(name.as_str()));
            close_self(state);
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
            state.show_keybindings = true;
        }
    }
}

/// Hide the plugin and reset state without restoring the origin tab.
/// Used when a command has already navigated to the desired destination.
fn close_self(state: &mut State) {
    state.search_term.clear();
    state.selected_index = 0;
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
