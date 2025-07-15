use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};
use std::collections::HashSet;

use crate::store_path::StorePathGraph;
use crate::ui::app::{App, Modal};

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
        Line::from("  w       Show why-depends (use h/l to scroll horizontally)"),
        Line::from("  s       Change sort order"),
        Line::from("  ?       Toggle this help"),
        Line::from("  q/Esc   Quit"),
        Line::from(""),
        Line::from("Press any key to close this help"),
    ];

    let block = Block::default()
        .title("Help")
        .borders(Borders::ALL)
        .style(Style::default().fg(Color::Black).bg(Color::White));

    let paragraph = Paragraph::new(help_text)
        .block(block)
        .style(Style::default().fg(Color::Black).bg(Color::White))
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
                        // Calculate it now using the nix-tree algorithm
                        // Use parent context for calculating added sizes
                        let parent_context = app.get_parent_context();
                        let added =
                            calculate_added_size_for_path(path, &app.graph, &parent_context);
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
                    Span::raw(format!("Immediate Parents ({parents_count}): ")),
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
    context_roots: &[String],
) -> u64 {
    // Following the original nix-tree logic:
    // addedSize = totalSize - filteredSize
    // where filteredSize = size of closure of (contextRoots - currentPath)

    // First, we need to calculate the total size of the current context
    // This is the closure of all items in the current view
    let mut context_closure = HashSet::new();
    for root in context_roots {
        let mut to_visit = vec![root.clone()];
        while let Some(current) = to_visit.pop() {
            if context_closure.insert(current.clone()) {
                if let Some(sp) = graph.get_path(&current) {
                    for reference in &sp.references {
                        if !context_closure.contains(reference) {
                            to_visit.push(reference.clone());
                        }
                    }
                }
            }
        }
    }

    let context_total_size: u64 = context_closure
        .iter()
        .filter_map(|p| graph.get_path(p))
        .map(|p| p.nar_size)
        .sum();

    // Build closure of all context roots, excluding current path and its descendants
    let mut filtered_closure = HashSet::new();

    for root in context_roots {
        // Skip current path when building the filtered closure
        if root != path {
            let mut to_visit = vec![root.clone()];
            while let Some(current) = to_visit.pop() {
                // Skip the target path completely - don't add it or traverse its children
                if current == path {
                    continue;
                }

                if filtered_closure.insert(current.clone()) {
                    if let Some(sp) = graph.get_path(&current) {
                        for reference in &sp.references {
                            if !filtered_closure.contains(reference) && reference != path {
                                to_visit.push(reference.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    // Calculate size of the filtered closure
    let filtered_size: u64 = filtered_closure
        .iter()
        .filter_map(|p| graph.get_path(p))
        .map(|p| p.nar_size)
        .sum();

    // Added size is context total minus filtered
    context_total_size.saturating_sub(filtered_size)
}

pub fn render_why_depends(
    f: &mut Frame,
    area: Rect,
    formatted_lines: &[String],
    max_line_width: usize,
    selected: usize,
    vertical_scroll_state: ScrollbarState,
    horizontal_scroll_state: ScrollbarState,
    horizontal_scroll: usize,
) {
    let modal_area = centered_rect(90, 60, area);

    // Clear with black background
    f.render_widget(Clear, modal_area);

    let block = Block::default()
        .title("Why Depends - Shows paths from roots to selected package")
        .borders(Borders::ALL);

    let inner_area = block.inner(modal_area);
    f.render_widget(block, modal_area);

    // Calculate visible window
    let visible_height = inner_area.height.saturating_sub(1) as usize; // Leave room for borders

    // Calculate scroll offset to keep selected item visible
    let scroll_offset = if visible_height > 0 && selected >= visible_height {
        selected.saturating_sub(visible_height / 2)
    } else {
        0
    };

    // Build visible lines
    let visible_lines = formatted_lines
        .iter()
        .enumerate()
        .skip(scroll_offset)
        .take(visible_height)
        .map(|(i, text)| {
            let style = if i == selected {
                Style::default().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };

            // Apply horizontal scroll by slicing the text safely at char boundaries
            let text_to_show = if horizontal_scroll < text.len() {
                // Skip the first `horizontal_scroll` characters safely
                text.chars().skip(horizontal_scroll).collect::<String>()
            } else {
                String::new()
            };

            Line::from(text_to_show).style(style)
        })
        .collect::<Vec<_>>();

    // Create paragraph
    let paragraph = Paragraph::new(visible_lines);
    f.render_widget(paragraph, inner_area);

    // Render vertical scrollbar if there are items to scroll
    if !formatted_lines.is_empty() && formatted_lines.len() > visible_height {
        let vertical_scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));

        let mut v_state = vertical_scroll_state;
        // Ensure the inner area calculation doesn't go negative
        if inner_area.height > 2 {
            f.render_stateful_widget(
                vertical_scrollbar,
                inner_area.inner(Margin {
                    vertical: 1,
                    horizontal: 0,
                }),
                &mut v_state,
            );
        }
    }

    // Render horizontal scrollbar if content is wider than view
    if max_line_width > inner_area.width as usize && inner_area.width > 2 {
        let horizontal_scrollbar = Scrollbar::new(ScrollbarOrientation::HorizontalBottom)
            .begin_symbol(Some("←"))
            .end_symbol(Some("→"));

        let mut h_state = horizontal_scroll_state;
        f.render_stateful_widget(
            horizontal_scrollbar,
            inner_area.inner(Margin {
                vertical: 0,
                horizontal: 1,
            }),
            &mut h_state,
        );
    }
}

pub fn render_modal(f: &mut Frame, app: &App, area: Rect) {
    if let Some(modal) = &app.modal {
        match modal {
            Modal::WhyDepends {
                paths: _,
                formatted_lines,
                max_line_width,
                selected,
                vertical_scroll_state,
                horizontal_scroll_state,
                horizontal_scroll,
            } => {
                render_why_depends(
                    f,
                    area,
                    formatted_lines,
                    *max_line_width,
                    *selected,
                    *vertical_scroll_state,
                    *horizontal_scroll_state,
                    *horizontal_scroll,
                );
            }
        }
    }
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
