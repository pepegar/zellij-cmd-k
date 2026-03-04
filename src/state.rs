use zellij_tile::prelude::*;

use crate::commands::ScoredCommand;

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Browse,
    TextInput(PendingAction),
}

impl Default for Mode {
    fn default() -> Self {
        Mode::Browse
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PendingAction {
    RenameTab { tab_position: u32 },
    RenamePane { pane_id: u32, is_plugin: bool },
}

#[derive(Default)]
pub struct State {
    pub tabs: Vec<TabInfo>,
    pub pane_manifest: Option<PaneManifest>,
    pub sessions: Vec<SessionInfo>,
    pub mode_info: Option<ModeInfo>,

    pub mode: Mode,
    pub search_term: String,
    pub selected_index: usize,
    pub input_buffer: String,
    pub filtered_commands: Vec<ScoredCommand>,
}
