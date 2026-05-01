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

    let title = format!("Current · sort: {}", app.sort_order.as_str());
    render_pane(
        f,
        chunks[1],
        &title,
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

    let inner_width = area.width.saturating_sub(2) as usize;
    const SIGN_W: usize = 2;

    let list_items: Vec<ListItem> = ctx
        .items
        .iter()
        .enumerate()
        .map(|(idx, path)| {
            let is_selected = ctx.state.selected() == Some(idx);
            let store_path = ctx.graph.get_path(path);
            let path_stats = ctx.stats.get(path);

            let name = store_path.map(|p| p.short_name()).unwrap_or(path.as_str());

            let size_str = path_stats
                .map(|s| format!("{:>10}", bytesize::ByteSize(s.closure_size)))
                .unwrap_or_default();

            let signed = store_path
                .map(|p| if p.is_signed() { "✓ " } else { "  " })
                .unwrap_or("  ");

            let name_budget = inner_width
                .saturating_sub(SIGN_W)
                .saturating_sub(size_str.len() + 1);
            let (name, pad) = fit_and_pad(name, name_budget);

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
                Span::raw(" ".repeat(pad + 1)),
                Span::styled(size_str, Style::default().fg(Color::Green)),
            ]);

            ListItem::new(line).style(style)
        })
        .collect();

    let position = ctx
        .state
        .selected()
        .map(|i| format!(" {}/{} ", i + 1, ctx.items.len()))
        .unwrap_or_default();

    let list = List::new(list_items).block(
        Block::default()
            .title(title)
            .title_bottom(Line::from(position).right_aligned())
            .borders(Borders::ALL)
            .border_style(border_style),
    );

    f.render_stateful_widget(list, area, &mut ctx.state.clone());
}

/// Truncate `s` to at most `budget` columns (appending `…` if cut) and report
/// remaining columns so the caller can right-align the next span. Store path
/// names are restricted to ASCII so byte length equals display width here.
fn fit_and_pad(s: &str, budget: usize) -> (String, usize) {
    if s.len() <= budget {
        return (s.to_string(), budget - s.len());
    }
    if budget == 0 {
        return (String::new(), 0);
    }
    let mut out: String = s.chars().take(budget - 1).collect();
    out.push('…');
    (out, 0)
}
