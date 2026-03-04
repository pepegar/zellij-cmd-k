use zellij_tile::prelude::*;

use crate::commands::Command;
use crate::state::{Mode, PendingAction, State};

pub fn handle_key(state: &mut State, key: KeyWithModifier) -> bool {
    match &state.mode {
        Mode::Browse => handle_browse_key(state, key),
        Mode::TextInput(_) => handle_text_input_key(state, key),
    }
}

fn handle_browse_key(state: &mut State, key: KeyWithModifier) -> bool {
    match key.bare_key {
        BareKey::Esc => {
            state.search_term.clear();
            state.selected_index = 0;
            hide_self();
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

fn handle_text_input_key(state: &mut State, key: KeyWithModifier) -> bool {
    match key.bare_key {
        BareKey::Esc => {
            state.mode = Mode::Browse;
            state.input_buffer.clear();
            true
        }
        BareKey::Enter => {
            let input = state.input_buffer.clone();
            if let Mode::TextInput(action) = &state.mode {
                match action {
                    PendingAction::RenameTab { tab_position } => {
                        rename_tab(*tab_position as u32, &input);
                    }
                    PendingAction::RenamePane { pane_id, is_plugin } => {
                        if *is_plugin {
                            rename_plugin_pane(*pane_id, &input);
                        } else {
                            rename_terminal_pane(*pane_id, &input);
                        }
                    }
                }
            }
            state.input_buffer.clear();
            state.mode = Mode::Browse;
            state.search_term.clear();
            state.selected_index = 0;
            hide_self();
            true
        }
        BareKey::Backspace => {
            state.input_buffer.pop();
            true
        }
        BareKey::Char(c) => {
            if !key.has_modifiers(&[KeyModifier::Ctrl])
                && !key.has_modifiers(&[KeyModifier::Alt])
            {
                state.input_buffer.push(c);
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
            // switch_tab_to is 1-indexed, TabInfo.position is 0-indexed
            switch_tab_to(*position as u32 + 1);
            dismiss(state);
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
            dismiss(state);
        }
        Command::NewTab => {
            new_tab(None::<&str>, None::<&str>);
            dismiss(state);
        }
        Command::NewPaneTiled => {
            open_terminal(std::path::PathBuf::from("."));
            dismiss(state);
        }
        Command::NewPaneFloating => {
            open_terminal_floating(std::path::PathBuf::from("."), None);
            dismiss(state);
        }
        Command::SwitchSession { name } => {
            switch_session(Some(name.as_str()));
        }
        Command::EnterScrollMode => {
            dismiss(state);
            switch_to_input_mode(&InputMode::Scroll);
        }
        Command::EnterSearchMode => {
            dismiss(state);
            switch_to_input_mode(&InputMode::EnterSearch);
        }
        // Commands that need secondary text input
        Command::RenameTab {
            position,
            current_name,
        } => {
            state.mode = Mode::TextInput(PendingAction::RenameTab {
                tab_position: *position as u32,
            });
            state.input_buffer = current_name.clone();
        }
        Command::RenamePane {
            id,
            is_plugin,
            current_title,
        } => {
            state.mode = Mode::TextInput(PendingAction::RenamePane {
                pane_id: *id,
                is_plugin: *is_plugin,
            });
            state.input_buffer = current_title.clone();
        }
    }
}

fn dismiss(state: &mut State) {
    state.search_term.clear();
    state.selected_index = 0;
    hide_self();
}
