use std::time::Duration;

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use zellij_tile::prelude::*;

#[derive(Debug, Clone)]
pub enum Command {
    SwitchToTab { name: String, position: usize },
    CloseTab { name: String, position: usize },
    SwitchSession { name: String },
    ResumeSession { name: String },
    NewSession,
    NewSessionPrompt,
    RenameSessionPrompt,
    EnterScrollMode,
    EnterSearchMode,
    ShowKeybindings,
}

impl Command {
    pub fn label(&self) -> String {
        match self {
            Command::SwitchToTab { name, position } => {
                format!("Switch to tab: {} ({})", name, position + 1)
            }
            Command::CloseTab { name, position } => {
                format!("Close tab: {} ({})", name, position + 1)
            }
            Command::SwitchSession { name } => format!("Switch to session: {}", name),
            Command::ResumeSession { name } => format!("Resume session: {}", name),
            Command::NewSession => "New session (auto-named)".to_string(),
            Command::NewSessionPrompt => "New or switch to session (by name)...".to_string(),
            Command::RenameSessionPrompt => "Rename current session...".to_string(),
            Command::EnterScrollMode => "Enter scroll mode".to_string(),
            Command::EnterSearchMode => "Enter search mode".to_string(),
            Command::ShowKeybindings => "Show all keybindings".to_string(),
        }
    }

    pub fn category(&self) -> &'static str {
        match self {
            Command::SwitchToTab { .. } | Command::CloseTab { .. } => "Tab",
            Command::SwitchSession { .. }
            | Command::ResumeSession { .. }
            | Command::NewSession
            | Command::NewSessionPrompt
            | Command::RenameSessionPrompt => "Session",
            Command::EnterScrollMode | Command::EnterSearchMode => "Mode",
            Command::ShowKeybindings => "Help",
        }
    }
}

pub struct ScoredCommand {
    pub command: Command,
    pub score: i64,
    pub match_indices: Vec<usize>,
}

pub fn build_commands(
    tabs: &[TabInfo],
    _pane_manifest: &Option<PaneManifest>,
    sessions: &[SessionInfo],
    resurrectable_sessions: &[(String, Duration)],
) -> Vec<Command> {
    let mut commands = Vec::new();

    // Static commands
    commands.push(Command::NewSession);
    commands.push(Command::NewSessionPrompt);
    commands.push(Command::RenameSessionPrompt);
    commands.push(Command::EnterScrollMode);
    commands.push(Command::EnterSearchMode);
    commands.push(Command::ShowKeybindings);

    // Tabs
    for tab in tabs {
        commands.push(Command::SwitchToTab {
            name: tab.name.clone(),
            position: tab.position,
        });
        commands.push(Command::CloseTab {
            name: tab.name.clone(),
            position: tab.position,
        });
    }

    // Live sessions (exclude current)
    for session in sessions {
        if !session.is_current_session {
            commands.push(Command::SwitchSession {
                name: session.name.clone(),
            });
        }
    }

    // Resurrectable sessions (skip any that are also live)
    for (name, _duration) in resurrectable_sessions {
        if sessions.iter().any(|s| &s.name == name) {
            continue;
        }
        commands.push(Command::ResumeSession { name: name.clone() });
    }

    commands
}

pub fn filter_commands(commands: &[Command], query: &str) -> Vec<ScoredCommand> {
    if query.is_empty() {
        return commands
            .iter()
            .map(|cmd| ScoredCommand {
                command: cmd.clone(),
                score: 0,
                match_indices: vec![],
            })
            .collect();
    }

    let matcher = SkimMatcherV2::default();
    let mut scored: Vec<ScoredCommand> = commands
        .iter()
        .filter_map(|cmd| {
            let label = cmd.label();
            matcher
                .fuzzy_indices(&label, query)
                .map(|(score, indices)| ScoredCommand {
                    command: cmd.clone(),
                    score,
                    match_indices: indices,
                })
        })
        .collect();

    scored.sort_by(|a, b| b.score.cmp(&a.score));
    scored
}
