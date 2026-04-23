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
        self.plugin_id = Some(get_plugin_ids().plugin_id);
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
                self.maybe_follow_user_tab();
                self.refilter();
                true
            }
            Event::PaneUpdate(manifest) => {
                self.pane_manifest = Some(manifest);
                self.maybe_follow_user_tab();
                self.refilter();
                true
            }
            Event::SessionUpdate(sessions, resurrectable) => {
                self.sessions = sessions;
                self.resurrectable_sessions = resurrectable;
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
    /// Zellij's `LaunchOrFocusPlugin` without `move_to_focused_tab true` warps the
    /// user to whichever tab the plugin was first loaded into. Detect that warp by
    /// comparing the plugin's current tab with the tab the user was last on, and
    /// move the plugin's pane over so the user stays where they were.
    fn maybe_follow_user_tab(&mut self) {
        let Some(plugin_id) = self.plugin_id else { return };
        let Some(manifest) = self.pane_manifest.as_ref() else { return };

        // Find the tab that contains our plugin pane and whether it's focused.
        let mut plugin_tab: Option<usize> = None;
        let mut plugin_focused = false;
        for (tab_pos, panes) in &manifest.panes {
            for pane in panes {
                if pane.is_plugin && pane.id == plugin_id {
                    plugin_tab = Some(*tab_pos);
                    plugin_focused = pane.is_focused;
                }
            }
        }
        let Some(plugin_tab) = plugin_tab else { return };
        let active_tab = self.tabs.iter().find(|t| t.active).map(|t| t.position);

        if plugin_focused && active_tab == Some(plugin_tab) {
            // Plugin visible. If the user was on a different tab just before, warp
            // our pane to that tab.
            if let Some(user_tab) = self.last_user_tab {
                if user_tab != plugin_tab {
                    break_panes_to_tab_with_index(
                        &[PaneId::Plugin(plugin_id)],
                        user_tab,
                        true,
                    );
                    // Clear so we don't loop on the follow-up updates.
                    self.last_user_tab = None;
                }
            }
        } else if let Some(active) = active_tab {
            // Plugin is hidden: remember where the user is so we can follow them
            // next time they invoke us.
            self.last_user_tab = Some(active);
        }
    }

    pub fn refilter(&mut self) {
        let all_commands = build_commands(
            &self.tabs,
            &self.pane_manifest,
            &self.sessions,
            &self.resurrectable_sessions,
        );
        self.filtered_commands = filter_commands(&all_commands, &self.search_term);
        if self.selected_index >= self.filtered_commands.len() {
            self.selected_index = self.filtered_commands.len().saturating_sub(1);
        }
    }
}
