use std::time::Duration;

use zellij_tile::prelude::*;

use crate::commands::ScoredCommand;

#[derive(Debug, Clone, Default)]
pub enum View {
    #[default]
    List,
    Keybindings { scroll: usize },
    NewSessionPrompt { input: String },
}

#[derive(Default)]
pub struct State {
    pub tabs: Vec<TabInfo>,
    pub pane_manifest: Option<PaneManifest>,
    pub sessions: Vec<SessionInfo>,
    pub resurrectable_sessions: Vec<(String, Duration)>,
    pub mode_info: Option<ModeInfo>,

    pub search_term: String,
    pub selected_index: usize,
    pub filtered_commands: Vec<ScoredCommand>,

    /// Tab that was focused when the plugin was last shown, so dismiss is a noop.
    pub origin_tab_position: Option<usize>,

    /// Which view is currently rendered. `View::List` is the default command list
    /// and search view; the other variants own their own scroll/input state.
    pub view: View,
}
