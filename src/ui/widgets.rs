use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

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
    let sort_info = format!("Sort: {} ", app.sort_order.as_str());
    let path_info = app.current_path.as_deref().unwrap_or("No selection");

    let status_line = Line::from(vec![
        Span::raw(path_info),
        Span::raw(" | "),
        Span::styled(sort_info, Style::default().fg(Color::Cyan)),
        Span::raw(" | "),
        Span::raw("Press ? for help"),
    ]);

    let paragraph = Paragraph::new(status_line).style(Style::default().bg(Color::DarkGray));

    f.render_widget(paragraph, area);
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
