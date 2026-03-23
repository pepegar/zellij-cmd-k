use zellij_tile::prelude::*;
use zellij_tile::prelude::actions::Action;

use crate::state::State;

pub fn render(state: &State, rows: usize, cols: usize) {
    if state.show_keybindings {
        render_keybindings(state, rows, cols);
        return;
    }

    // Row 0: search input
    // Row 1..rows-1: command list
    // Last row: hint bar

    let search_display = if state.search_term.is_empty() {
        " > Type to filter...".to_string()
    } else {
        format!(" > {}|", state.search_term)
    };
    let search_text = Text::new(&search_display).color_range(3, 0..2);
    print_text_with_coordinates(search_text, 0, 0, Some(cols), Some(1));

    let list_start = 1;
    let hint_row = rows.saturating_sub(1);
    let list_height = hint_row.saturating_sub(list_start);

    if list_height == 0 || state.filtered_commands.is_empty() {
        if state.filtered_commands.is_empty() && !state.search_term.is_empty() {
            let no_match = Text::new(" No matches");
            print_text_with_coordinates(no_match, 0, list_start, Some(cols), Some(1));
        }
        let hint = Text::new(" Esc close");
        print_text_with_coordinates(hint, 0, hint_row, Some(cols), Some(1));
        return;
    }

    let scroll_offset =
        compute_scroll_offset(state.selected_index, list_height, state.filtered_commands.len());

    let visible = state
        .filtered_commands
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .take(list_height);

    let mut items: Vec<NestedListItem> = Vec::new();

    for (i, scored) in visible {
        let category = scored.command.category();
        let label = scored.command.label();
        let display = format!("[{}] {}", category, label);

        let mut item = NestedListItem::new(&display);

        // Color the category bracket
        let cat_end = category.len() + 2; // "[Category]"
        item = item.color_range(2, 0..cat_end);

        // Highlight fuzzy match indices (offset by category prefix length + 1 for space)
        let prefix_len = cat_end + 1;
        for &idx in &scored.match_indices {
            let display_idx = prefix_len + idx;
            if display_idx < display.len() {
                item = item.color_indices(1, vec![display_idx]);
            }
        }

        if i == state.selected_index {
            item = item.selected();
        }

        items.push(item);
    }

    print_nested_list_with_coordinates(items, 0, list_start, Some(cols), Some(list_height));

    // Hint bar
    let hint = Text::new(" Enter select | Esc close | Up/Down navigate | ? keybindings");
    print_text_with_coordinates(hint, 0, hint_row, Some(cols), Some(1));
}

fn compute_scroll_offset(selected: usize, visible_height: usize, total: usize) -> usize {
    if total <= visible_height {
        return 0;
    }
    if selected < visible_height / 2 {
        return 0;
    }
    let max_offset = total.saturating_sub(visible_height);
    let ideal = selected.saturating_sub(visible_height / 2);
    ideal.min(max_offset)
}

fn render_keybindings(state: &State, rows: usize, cols: usize) {
    let title = Text::new(" ⌨  Zellij Keybindings").color_range(3, 0..22);
    print_text_with_coordinates(title, 0, 0, Some(cols), Some(1));

    let separator = Text::new(&"─".repeat(cols.min(60)));
    print_text_with_coordinates(separator, 0, 1, Some(cols), Some(1));

    let hint_row = rows.saturating_sub(1);
    let list_start = 2;
    let list_height = hint_row.saturating_sub(list_start);

    let bindings = collect_keybindings(state);

    // Clamp scroll offset
    let max_scroll = bindings.len().saturating_sub(list_height);
    let scroll = state.keybindings_scroll.min(max_scroll);

    let visible = bindings.iter().skip(scroll).take(list_height);
    let mut items: Vec<NestedListItem> = Vec::new();

    for entry in visible {
        match entry {
            KeybindingEntry::SectionHeader(header) => {
                let item = NestedListItem::new(header).color_range(2, 0..header.len());
                items.push(item);
            }
            KeybindingEntry::Binding(display) => {
                items.push(NestedListItem::new(display));
            }
        }
    }

    if !items.is_empty() {
        print_nested_list_with_coordinates(items, 0, list_start, Some(cols), Some(list_height));
    }

    let hint = Text::new(" Esc back | Up/Down scroll");
    print_text_with_coordinates(hint, 0, hint_row, Some(cols), Some(1));
}

enum KeybindingEntry {
    SectionHeader(String),
    Binding(String),
}

fn collect_keybindings(state: &State) -> Vec<KeybindingEntry> {
    let mut entries = Vec::new();

    let mode_info = match &state.mode_info {
        Some(mi) => mi,
        None => {
            entries.push(KeybindingEntry::Binding(
                "  (keybinding info not available yet)".to_string(),
            ));
            return entries;
        }
    };

    // Modes to display, in a sensible order
    let modes_order = [
        InputMode::Normal,
        InputMode::Locked,
        InputMode::Pane,
        InputMode::Tab,
        InputMode::Resize,
        InputMode::Move,
        InputMode::Scroll,
        InputMode::Session,
    ];

    for mode in &modes_order {
        // keybinds is Vec<(InputMode, Vec<(KeyWithModifier, Vec<Action>)>)>
        if let Some((_m, mode_keybinds)) = mode_info.keybinds.iter().find(|(m, _)| m == mode) {
            let mode_name = format!("{:?}", mode);
            entries.push(KeybindingEntry::SectionHeader(format!("┌ {} mode", mode_name)));

            for (key, actions) in mode_keybinds {
                let key_str = format_key(key);
                let actions_str: Vec<String> = actions.iter().map(|a| format_action(a)).collect();
                entries.push(KeybindingEntry::Binding(format!(
                    "  {} → {}",
                    key_str,
                    actions_str.join(", ")
                )));
            }
        }
    }

    // Catch any modes we didn't list above
    for (mode, mode_keybinds) in &mode_info.keybinds {
        if modes_order.contains(mode) {
            continue;
        }
        let mode_name = format!("{:?}", mode);
        entries.push(KeybindingEntry::SectionHeader(format!("┌ {} mode", mode_name)));

        for (key, actions) in mode_keybinds {
            let key_str = format_key(key);
            let actions_str: Vec<String> = actions.iter().map(|a| format_action(a)).collect();
            entries.push(KeybindingEntry::Binding(format!(
                "  {} → {}",
                key_str,
                actions_str.join(", ")
            )));
        }
    }

    entries
}

fn format_key(key: &KeyWithModifier) -> String {
    let mut parts = Vec::new();
    if key.has_modifiers(&[KeyModifier::Ctrl]) {
        parts.push("Ctrl");
    }
    if key.has_modifiers(&[KeyModifier::Alt]) {
        parts.push("Alt");
    }
    if key.has_modifiers(&[KeyModifier::Super]) {
        parts.push("Super");
    }
    let base = match &key.bare_key {
        BareKey::Char(c) => {
            if *c == ' ' {
                "Space".to_string()
            } else {
                c.to_string()
            }
        }
        other => format!("{:?}", other),
    };
    parts.push(&base);
    // We need to own the string, so collect into owned strings
    let owned: Vec<String> = parts.iter().map(|s| s.to_string()).collect();
    owned.join("+")
}

fn format_action(action: &Action) -> String {
    format!("{:?}", action)
}
