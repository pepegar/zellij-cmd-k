use std::collections::BTreeMap;
use zellij_tile::prelude::*;

mod commands;
mod input;
mod render;
mod state;

use commands::{build_commands, filter_commands};
use state::State;

register_plugin!(State);

impl ZellijPlugin for State {
    fn load(&mut self, _configuration: BTreeMap<String, String>) {
        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
            PermissionType::OpenTerminalsOrPlugins,
        ]);
        subscribe(&[
            EventType::TabUpdate,
            EventType::PaneUpdate,
            EventType::SessionUpdate,
            EventType::ModeUpdate,
            EventType::Key,
        ]);
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::TabUpdate(tabs) => {
                // Track which tab is currently focused so dismiss can restore it.
                self.origin_tab_position = tabs
                    .iter()
                    .find(|t| t.active)
                    .map(|t| t.position);
                self.tabs = tabs;
                self.refilter();
                true
            }
            Event::PaneUpdate(manifest) => {
                self.pane_manifest = Some(manifest);
                self.refilter();
                true
            }
            Event::SessionUpdate(sessions, _durations) => {
                self.sessions = sessions;
                self.refilter();
                true
            }
            Event::ModeUpdate(mode_info) => {
                self.mode_info = Some(mode_info);
                false
            }
            Event::Key(key) => input::handle_key(self, key),
            _ => false,
        }
    }

    fn render(&mut self, rows: usize, cols: usize) {
        render::render(self, rows, cols);
    }
}

impl State {
    pub fn refilter(&mut self) {
        let all_commands = build_commands(&self.tabs, &self.pane_manifest, &self.sessions);
        self.filtered_commands = filter_commands(&all_commands, &self.search_term);
        if self.selected_index >= self.filtered_commands.len() {
            self.selected_index = self.filtered_commands.len().saturating_sub(1);
        }
    }
}
