use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
};

use crate::path_stats::PathStats;
use crate::store_path::StorePathGraph;
use crate::ui::app::{App, Pane};
use std::collections::HashMap;

pub fn render_panes(f: &mut Frame, app: &App, area: Rect) {
    let chunks = ratatui::layout::Layout::horizontal([
        Constraint::Percentage(30),
        Constraint::Percentage(40),
        Constraint::Percentage(30),
    ])
    .split(area);

    render_pane(
        f,
        chunks[0],
        "Referrers",
        &PaneRenderContext {
            items: &app.previous_items,
            state: &app.previous_state,
            is_active: app.active_pane == Pane::Previous,
            graph: &app.graph,
            stats: &app.stats,
        },
    );

    render_pane(
        f,
        chunks[1],
        "Current",
        &PaneRenderContext {
            items: &app.current_items,
            state: &app.current_state,
            is_active: app.active_pane == Pane::Current,
            graph: &app.graph,
            stats: &app.stats,
        },
    );

    render_pane(
        f,
        chunks[2],
        "Dependencies",
        &PaneRenderContext {
            items: &app.next_items,
            state: &app.next_state,
            is_active: app.active_pane == Pane::Next,
            graph: &app.graph,
            stats: &app.stats,
        },
    );
}

struct PaneRenderContext<'a> {
    items: &'a [String],
    state: &'a ratatui::widgets::ListState,
    is_active: bool,
    graph: &'a StorePathGraph,
    stats: &'a HashMap<String, PathStats>,
}

fn render_pane(f: &mut Frame, area: Rect, title: &str, ctx: &PaneRenderContext) {
    let border_style = if ctx.is_active {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let list_items: Vec<ListItem> = ctx
        .items
        .iter()
        .enumerate()
        .map(|(idx, path)| {
            let is_selected = ctx.state.selected() == Some(idx);
            let store_path = ctx.graph.get_path(path);
            let path_stats = ctx.stats.get(path);

            let name = store_path.map(|p| p.short_name()).unwrap_or(path.as_str());

            let size_str = if let Some(stats) = path_stats {
                format!(" ({})", bytesize::ByteSize(stats.closure_size))
            } else {
                String::new()
            };

            let signed = store_path
                .map(|p| if p.is_signed() { "✓ " } else { "  " })
                .unwrap_or("  ");

            let style = if is_selected && ctx.is_active {
                Style::default()
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD)
            } else if is_selected {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };

            let line = Line::from(vec![
                Span::styled(signed, Style::default().fg(Color::Cyan)),
                Span::raw(name),
                Span::styled(size_str, Style::default().fg(Color::Green)),
            ]);

            ListItem::new(line).style(style)
        })
        .collect();

    let list = List::new(list_items).block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(border_style),
    );

    f.render_stateful_widget(list, area, &mut ctx.state.clone());
}
