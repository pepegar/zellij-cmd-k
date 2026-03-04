use zellij_tile::prelude::*;

use crate::commands::ScoredCommand;

#[derive(Default)]
pub struct State {
    pub tabs: Vec<TabInfo>,
    pub pane_manifest: Option<PaneManifest>,
    pub sessions: Vec<SessionInfo>,
    pub mode_info: Option<ModeInfo>,

    pub search_term: String,
    pub selected_index: usize,
    pub filtered_commands: Vec<ScoredCommand>,

    /// Tab that was focused when the plugin was last shown, so dismiss is a noop.
    pub origin_tab_position: Option<usize>,
}
