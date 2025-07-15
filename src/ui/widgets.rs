use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};
use std::collections::{HashMap, HashSet};

use crate::path_stats::PathStats;
use crate::store_path::StorePathGraph;
use crate::ui::app::App;

pub fn render_help(f: &mut Frame, area: Rect) {
    let help_text = vec![
        Line::from("nix-tree - Interactive Nix dependency viewer"),
        Line::from(""),
        Line::from("Navigation:"),
        Line::from("  j/↓     Move down"),
        Line::from("  k/↑     Move up"),
        Line::from("  h/←     Move to previous pane"),
        Line::from("  l/→     Move to next pane"),
        Line::from("  Enter   Select item"),
        Line::from(""),
        Line::from("Actions:"),
        Line::from("  /       Search"),
        Line::from("  s       Change sort order"),
        Line::from("  ?       Toggle this help"),
        Line::from("  q/Esc   Quit"),
        Line::from(""),
        Line::from("Press any key to close this help"),
    ];

    let block = Block::default()
        .title("Help")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::White));

    let paragraph = Paragraph::new(help_text)
        .block(block)
        .alignment(Alignment::Left);

    let help_area = centered_rect(60, 70, area);
    f.render_widget(Clear, help_area);
    f.render_widget(paragraph, help_area);
}

pub fn render_search(f: &mut Frame, area: Rect, query: &str) {
    let search_text = vec![Line::from("Search:"), Line::from(query)];

    let block = Block::default()
        .title("Search")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Yellow));

    let paragraph = Paragraph::new(search_text)
        .block(block)
        .alignment(Alignment::Left);

    let search_area = centered_rect(50, 20, area);
    f.render_widget(Clear, search_area);
    f.render_widget(paragraph, search_area);
}

pub fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    if let Some(path) = &app.current_path {
        // First line: full path
        let path_line = Line::from(vec![Span::raw(path)]);

        // Second line: detailed stats
        if let Some(store_path) = app.graph.get_path(path) {
            let stats = app.stats.get(path);

            let nar_size = bytesize::ByteSize(store_path.nar_size);
            let closure_size = stats
                .map(|s| bytesize::ByteSize(s.closure_size))
                .unwrap_or(bytesize::ByteSize(0));
            // Calculate added size on-demand if not already calculated
            let added_size = if let Some(s) = stats {
                match s.added_size {
                    Some(size) => bytesize::ByteSize(size),
                    None => {
                        // Calculate it now
                        let added = calculate_added_size_for_path(path, &app.graph, &app.stats);
                        bytesize::ByteSize(added)
                    }
                }
            } else {
                bytesize::ByteSize(0)
            };

            let signatures = if store_path.signatures.is_empty() {
                "none".to_string()
            } else {
                store_path.signatures.join(", ")
            };

            let parents_count = stats.map(|s| s.immediate_parents.len()).unwrap_or(0);
            let parents_preview = stats
                .map(|s| {
                    let names: Vec<String> = s
                        .immediate_parents
                        .iter()
                        .take(5)
                        .filter_map(|p| app.graph.get_path(p))
                        .map(|sp| sp.short_name().to_string())
                        .collect();
                    if s.immediate_parents.len() > 5 {
                        format!("{}, ...", names.join(", "))
                    } else {
                        names.join(", ")
                    }
                })
                .unwrap_or_default();

            let stats_line = Line::from(vec![
                Span::raw("NAR Size: "),
                Span::styled(nar_size.to_string(), Style::default().fg(Color::Yellow)),
                Span::raw(" | Closure Size: "),
                Span::styled(closure_size.to_string(), Style::default().fg(Color::Green)),
                Span::raw(" | Added Size: "),
                Span::styled(added_size.to_string(), Style::default().fg(Color::Cyan)),
            ]);

            let info_line = Line::from(vec![
                Span::raw("Signatures: "),
                Span::styled(signatures, Style::default().fg(Color::Magenta)),
            ]);

            let parents_line = if parents_count > 0 {
                Line::from(vec![
                    Span::raw(format!("Immediate Parents ({}): ", parents_count)),
                    Span::styled(parents_preview, Style::default().fg(Color::Blue)),
                ])
            } else {
                Line::from(vec![Span::raw("Immediate Parents: none")])
            };

            let text = vec![path_line, stats_line, info_line, parents_line];
            let paragraph = Paragraph::new(text);
            f.render_widget(paragraph, area);
        } else {
            // Fallback to simple display
            let status_line = Line::from(vec![
                Span::raw(path),
                Span::raw(" | Sort: "),
                Span::raw(app.sort_order.as_str()),
                Span::raw(" | Press ? for help"),
            ]);
            let paragraph = Paragraph::new(status_line);
            f.render_widget(paragraph, area);
        }
    } else {
        let status_line = Line::from(vec![Span::raw("No selection | Press ? for help")]);
        let paragraph = Paragraph::new(status_line);
        f.render_widget(paragraph, area);
    }
}

fn calculate_added_size_for_path(
    path: &str,
    graph: &StorePathGraph,
    stats: &HashMap<String, PathStats>,
) -> u64 {
    // Quick calculation of added size for a single path
    let Some(_store_path) = graph.get_path(path) else {
        return 0;
    };

    // Build closure for this path
    let mut closure = HashSet::new();
    let mut to_visit = vec![path.to_string()];

    while let Some(current) = to_visit.pop() {
        if closure.insert(current.clone()) {
            if let Some(sp) = graph.get_path(&current) {
                for reference in &sp.references {
                    if !closure.contains(reference) {
                        to_visit.push(reference.clone());
                    }
                }
            }
        }
    }

    // Get all siblings that share the same parents
    let mut shared_with_siblings = HashSet::new();
    if let Some(path_stats) = stats.get(path) {
        for parent in &path_stats.immediate_parents {
            if let Some(parent_path) = graph.get_path(parent) {
                for sibling_ref in &parent_path.references {
                    if sibling_ref != path {
                        // Add sibling's closure
                        let mut sibling_to_visit = vec![sibling_ref.clone()];
                        while let Some(current) = sibling_to_visit.pop() {
                            if shared_with_siblings.insert(current.clone()) {
                                if let Some(sp) = graph.get_path(&current) {
                                    for reference in &sp.references {
                                        if !shared_with_siblings.contains(reference) {
                                            sibling_to_visit.push(reference.clone());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Calculate unique size
    let unique_to_path: HashSet<_> = closure.difference(&shared_with_siblings).cloned().collect();

    unique_to_path
        .iter()
        .filter_map(|p| graph.get_path(p))
        .map(|p| p.nar_size)
        .sum()
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = ratatui::layout::Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(r);

    ratatui::layout::Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}
