use zellij_tile::prelude::*;

use crate::commands::Command;
use crate::state::State;

pub fn handle_key(state: &mut State, key: KeyWithModifier) -> bool {
    match key.bare_key {
        BareKey::Esc => {
            dismiss(state);
            true
        }
        BareKey::Enter => {
            if let Some(scored) = state.filtered_commands.get(state.selected_index) {
                let cmd = scored.command.clone();
                execute_command(state, &cmd);
            }
            true
        }
        BareKey::Up => {
            state.selected_index = state.selected_index.saturating_sub(1);
            true
        }
        BareKey::Down => {
            if state.selected_index + 1 < state.filtered_commands.len() {
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
            if !key.has_modifiers(&[KeyModifier::Ctrl])
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
        Command::SwitchToPane {
            id,
            tab_position,
            is_plugin,
            is_floating,
            ..
        } => {
            switch_tab_to(*tab_position as u32 + 1);
            if *is_plugin {
                focus_plugin_pane(*id, *is_floating);
            } else {
                focus_terminal_pane(*id, *is_floating);
            }
            close_self(state);
        }
        Command::ClosePane { id, is_plugin, .. } => {
            if *is_plugin {
                close_plugin_pane(*id);
            } else {
                close_terminal_pane(*id);
            }
            close_self(state);
        }
        Command::NewTab => {
            new_tab(None::<&str>, None::<&str>);
            close_self(state);
        }
        Command::NewPaneTiled => {
            open_terminal(std::path::PathBuf::from("."));
            close_self(state);
        }
        Command::NewPaneFloating => {
            open_terminal_floating(std::path::PathBuf::from("."), None);
            close_self(state);
        }
        Command::SwitchSession { name } => {
            switch_session(Some(name.as_str()));
        }
        Command::EnterScrollMode => {
            close_self(state);
            switch_to_input_mode(&InputMode::Scroll);
        }
        Command::EnterSearchMode => {
            close_self(state);
            switch_to_input_mode(&InputMode::EnterSearch);
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
