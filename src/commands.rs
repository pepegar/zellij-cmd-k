use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use zellij_tile::prelude::*;

#[derive(Debug, Clone)]
pub enum Command {
    SwitchToTab { name: String, position: usize },
    CloseTab { name: String, position: usize },
    SwitchSession { name: String },
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
            Command::SwitchSession { name } => format!("Go to session: {}", name),
            Command::EnterScrollMode => "Enter scroll mode".to_string(),
            Command::EnterSearchMode => "Enter search mode".to_string(),
            Command::ShowKeybindings => "Show all keybindings".to_string(),
        }
    }

    pub fn category(&self) -> &'static str {
        match self {
            Command::SwitchToTab { .. } | Command::CloseTab { .. } => "Tab",
            Command::SwitchSession { .. } => "Session",
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
) -> Vec<Command> {
    let mut commands = Vec::new();

    // Static commands
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

    // Sessions (exclude current)
    for session in sessions {
        if !session.is_current_session {
            commands.push(Command::SwitchSession {
                name: session.name.clone(),
            });
        }
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
