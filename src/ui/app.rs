use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::widgets::ListState;
use std::collections::HashMap;

use crate::path_stats::{PathStats, SortOrder};
use crate::store_path::StorePathGraph;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Previous,
    Current,
    Next,
}

impl Pane {}

pub enum Modal {
    WhyDepends {
        paths: Vec<Vec<String>>,
        formatted_lines: Vec<String>,
        max_line_width: usize,
        selected: usize,
        vertical_scroll_state: ratatui::widgets::ScrollbarState,
        horizontal_scroll_state: ratatui::widgets::ScrollbarState,
        horizontal_scroll: usize,
    },
}

pub struct App {
    pub graph: StorePathGraph,
    pub stats: HashMap<String, PathStats>,
    pub sort_order: SortOrder,
    pub active_pane: Pane,
    pub show_help: bool,
    pub searching: bool,
    pub search_query: String,

    pub previous_state: ListState,
    pub current_state: ListState,
    pub next_state: ListState,

    pub previous_items: Vec<String>,
    pub current_items: Vec<String>,
    pub next_items: Vec<String>,

    pub current_path: Option<String>,

    // Navigation history: (items, selected_index)
    pub navigation_history: Vec<(Vec<String>, Option<usize>)>,

    pub modal: Option<Modal>,
}

impl App {
    pub fn get_parent_context(&self) -> Vec<String> {
        // Get the parent context from navigation history
        // For added size calculation, we need the specific parent we navigated from
        if let Some((parent_items, selected_idx)) = self.navigation_history.last() {
            // If we have a selected parent, use only that as the context
            if let Some(idx) = selected_idx {
                if let Some(parent) = parent_items.get(*idx) {
                    return vec![parent.clone()];
                }
            }
            // Fallback to all parent items
            parent_items.clone()
        } else {
            // If no history, use the original roots
            self.graph.roots.clone()
        }
    }

    pub fn new(graph: StorePathGraph, stats: HashMap<String, PathStats>) -> Self {
        let mut app = Self {
            graph,
            stats,
            sort_order: SortOrder::ClosureSize,
            active_pane: Pane::Current,
            show_help: false,
            searching: false,
            search_query: String::new(),
            previous_state: ListState::default(),
            current_state: ListState::default(),
            next_state: ListState::default(),
            previous_items: Vec::new(),
            current_items: Vec::new(),
            next_items: Vec::new(),
            current_path: None,
            navigation_history: Vec::new(),
            modal: None,
        };

        // Start with all roots in the current pane
        app.current_items = app.graph.roots.clone();
        crate::path_stats::sort_paths(&mut app.current_items, &app.stats, app.sort_order);

        if !app.current_items.is_empty() {
            app.current_state.select(Some(0));
            app.update_panes();
        }

        app
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Result<bool> {
        // Handle modal first
        if let Some(modal) = &mut self.modal {
            match modal {
                Modal::WhyDepends {
                    paths,
                    formatted_lines,
                    max_line_width,
                    selected,
                    vertical_scroll_state,
                    horizontal_scroll_state,
                    horizontal_scroll,
                } => {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            self.modal = None;
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if *selected < formatted_lines.len().saturating_sub(1) {
                                *selected += 1;
                                *vertical_scroll_state = vertical_scroll_state
                                    .content_length(formatted_lines.len())
                                    .position(*selected);
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if *selected > 0 {
                                *selected -= 1;
                                *vertical_scroll_state = vertical_scroll_state
                                    .content_length(formatted_lines.len())
                                    .position(*selected);
                            }
                        }
                        KeyCode::Enter => {
                            if let Some(path) = paths.get(*selected).cloned() {
                                self.modal = None;
                                self.select_path_from_why_depends(path);
                                return Ok(false);
                            }
                        }
                        KeyCode::Left | KeyCode::Char('h') => {
                            *horizontal_scroll = horizontal_scroll.saturating_sub(5);
                            *horizontal_scroll_state = horizontal_scroll_state
                                .content_length(*max_line_width)
                                .position(*horizontal_scroll);
                        }
                        KeyCode::Right | KeyCode::Char('l') => {
                            // Don't scroll beyond the longest line
                            let max_scroll = max_line_width.saturating_sub(20); // Leave some buffer
                            *horizontal_scroll = (*horizontal_scroll + 5).min(max_scroll);
                            *horizontal_scroll_state = horizontal_scroll_state
                                .content_length(*max_line_width)
                                .position(*horizontal_scroll);
                        }
                        KeyCode::PageDown => {
                            let page_size = 10; // Adjust based on your modal height
                            *selected = (*selected + page_size)
                                .min(formatted_lines.len().saturating_sub(1));
                            *vertical_scroll_state = vertical_scroll_state
                                .content_length(formatted_lines.len())
                                .position(*selected);
                        }
                        KeyCode::PageUp => {
                            let page_size = 10;
                            *selected = selected.saturating_sub(page_size);
                            *vertical_scroll_state = vertical_scroll_state
                                .content_length(formatted_lines.len())
                                .position(*selected);
                        }
                        _ => {}
                    }
                }
            }
            return Ok(false);
        }

        if self.searching {
            match key.code {
                KeyCode::Esc => {
                    self.searching = false;
                    self.search_query.clear();
                }
                KeyCode::Enter => {
                    self.searching = false;
                    self.perform_search();
                }
                KeyCode::Backspace => {
                    self.search_query.pop();
                }
                KeyCode::Char(c) => {
                    self.search_query.push(c);
                }
                _ => {}
            }
            return Ok(false);
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return Ok(true),
            KeyCode::Char('?') => self.show_help = !self.show_help,
            KeyCode::Char('/') => {
                self.searching = true;
                self.search_query.clear();
            }
            KeyCode::Char('w') => self.show_why_depends(),
            KeyCode::Char('s') => {
                self.sort_order = self.sort_order.next();
                self.resort_current_pane();
            }
            KeyCode::Down | KeyCode::Char('j') => self.move_down(),
            KeyCode::Up | KeyCode::Char('k') => self.move_up(),
            KeyCode::Left | KeyCode::Char('h') => self.move_left(),
            KeyCode::Right | KeyCode::Char('l') => self.move_right(),
            KeyCode::Enter => self.select_item(),
            _ => {}
        }

        Ok(false)
    }

    fn move_down(&mut self) {
        // Navigate items in the current pane
        let items = &self.current_items;
        if !items.is_empty() {
            let i = match self.current_state.selected() {
                Some(i) => (i + 1).min(items.len() - 1),
                None => 0,
            };
            self.current_state.select(Some(i));
            self.update_panes();
        }
    }

    fn move_up(&mut self) {
        // Navigate items in the current pane
        if let Some(i) = self.current_state.selected() {
            if i > 0 {
                self.current_state.select(Some(i - 1));
                self.update_panes();
            }
        }
    }

    fn move_left(&mut self) {
        // Go back in navigation history
        if let Some((items, selected_idx)) = self.navigation_history.pop() {
            self.current_items = items;
            self.current_state = ListState::default();
            if let Some(idx) = selected_idx {
                self.current_state.select(Some(idx));
            }
            self.update_panes();
        }
    }

    fn move_right(&mut self) {
        // Ranger-style: move all dependencies to current pane
        if !self.next_items.is_empty() {
            // Save current state to history
            let current_selection = self.current_state.selected();
            self.navigation_history
                .push((self.current_items.clone(), current_selection));

            // Move all dependencies to become the new current items
            self.current_items = self.next_items.clone();
            self.current_state.select(Some(0));
            self.update_panes();
        }
    }

    fn select_item(&mut self) {
        // Enter key behavior: update the panes based on selected item
        self.update_panes();
    }

    fn update_panes(&mut self) {
        // Use the selected item in current_items as the focus
        let selected_idx = self.current_state.selected().unwrap_or(0);
        if let Some(path) = self.current_items.get(selected_idx) {
            self.current_path = Some(path.clone());

            // Update referrers (left pane)
            self.previous_items = self
                .stats
                .get(path)
                .map(|s| s.immediate_parents.clone())
                .unwrap_or_default();
            crate::path_stats::sort_paths(&mut self.previous_items, &self.stats, self.sort_order);

            // Update dependencies (right pane)
            let mut refs = self
                .graph
                .get_references(path)
                .into_iter()
                .map(|p| p.path.clone())
                .collect::<Vec<_>>();
            crate::path_stats::sort_paths(&mut refs, &self.stats, self.sort_order);
            self.next_items = refs;

            // Reset selections in side panes but keep current pane focus
            self.previous_state = ListState::default();
            self.next_state = ListState::default();

            // Select first item in each pane if available
            if !self.previous_items.is_empty() {
                self.previous_state.select(Some(0));
            }
            if !self.next_items.is_empty() {
                self.next_state.select(Some(0));
            }

            // Always keep focus on current pane
            self.active_pane = Pane::Current;
        }
    }

    fn resort_current_pane(&mut self) {
        crate::path_stats::sort_paths(&mut self.current_items, &self.stats, self.sort_order);
        crate::path_stats::sort_paths(&mut self.previous_items, &self.stats, self.sort_order);
        crate::path_stats::sort_paths(&mut self.next_items, &self.stats, self.sort_order);
    }

    fn perform_search(&mut self) {
        if self.search_query.is_empty() {
            return;
        }

        let query = self.search_query.to_lowercase();
        let matching_paths: Vec<String> = self
            .graph
            .paths
            .iter()
            .filter(|p| p.name.to_lowercase().contains(&query))
            .map(|p| p.path.clone())
            .collect();

        if !matching_paths.is_empty() {
            self.current_items = matching_paths;
            crate::path_stats::sort_paths(&mut self.current_items, &self.stats, self.sort_order);
            self.current_state.select(Some(0));
            self.active_pane = Pane::Current;
            self.update_panes();
        }
    }

    fn show_why_depends(&mut self) {
        if let Some(path) = &self.current_path {
            let paths = crate::path_stats::why_depends(&self.graph, path);
            if !paths.is_empty() {
                // Pre-format all lines to avoid recomputing on every render
                let mut formatted_lines = Vec::new();
                let mut max_line_width = 0;

                for path in &paths {
                    let text = path
                        .iter()
                        .map(|p| {
                            // Extract short name from path
                            p.split('-').skip(1).collect::<Vec<_>>().join("-")
                        })
                        .collect::<Vec<_>>()
                        .join(" → ");

                    // Count characters instead of bytes for proper UTF-8 handling
                    max_line_width = max_line_width.max(text.chars().count());
                    formatted_lines.push(text);
                }

                let vertical_scroll_state = ratatui::widgets::ScrollbarState::default()
                    .content_length(formatted_lines.len())
                    .position(0);

                let horizontal_scroll_state =
                    ratatui::widgets::ScrollbarState::default().content_length(max_line_width);

                self.modal = Some(Modal::WhyDepends {
                    paths,
                    formatted_lines,
                    max_line_width,
                    selected: 0,
                    vertical_scroll_state,
                    horizontal_scroll_state,
                    horizontal_scroll: 0,
                });
            }
        }
    }

    fn select_path_from_why_depends(&mut self, path: Vec<String>) {
        // Clear navigation history
        self.navigation_history.clear();

        // Start from roots
        self.current_items = self.graph.roots.clone();
        crate::path_stats::sort_paths(&mut self.current_items, &self.stats, self.sort_order);

        // Navigate through the path
        for (i, target) in path.iter().enumerate() {
            // Find the target in current items
            if let Some(idx) = self.current_items.iter().position(|p| p == target) {
                self.current_state.select(Some(idx));
                self.update_panes();

                // If not the last item, move right to continue navigation
                if i < path.len() - 1 {
                    self.move_right();
                }
            }
        }
    }
}
