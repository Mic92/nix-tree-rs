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

impl Pane {
    pub fn previous(&self) -> Self {
        match self {
            Pane::Previous => Pane::Previous,
            Pane::Current => Pane::Previous,
            Pane::Next => Pane::Current,
        }
    }
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
}

impl App {
    pub fn new(graph: StorePathGraph, stats: HashMap<String, PathStats>) -> Self {
        let mut app = Self {
            graph,
            stats,
            sort_order: SortOrder::Alphabetical,
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
        };

        app.current_items = app.graph.roots.clone();
        crate::path_stats::sort_paths(&mut app.current_items, &app.stats, app.sort_order);

        if !app.current_items.is_empty() {
            app.current_state.select(Some(0));
            app.update_panes();
        }

        app
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Result<bool> {
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
            KeyCode::Char('s') => {
                self.sort_order = self.sort_order.next();
                self.resort_current_pane();
            }
            KeyCode::Down | KeyCode::Char('j') => self.move_down(),
            KeyCode::Up | KeyCode::Char('k') => self.move_up(),
            KeyCode::Left | KeyCode::Char('h') => self.move_left(),
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Enter => self.move_right(),
            _ => {}
        }

        Ok(false)
    }

    fn move_down(&mut self) {
        let state = match self.active_pane {
            Pane::Previous => &mut self.previous_state,
            Pane::Current => &mut self.current_state,
            Pane::Next => &mut self.next_state,
        };

        let items = match self.active_pane {
            Pane::Previous => &self.previous_items,
            Pane::Current => &self.current_items,
            Pane::Next => &self.next_items,
        };

        if !items.is_empty() {
            let i = match state.selected() {
                Some(i) => (i + 1).min(items.len() - 1),
                None => 0,
            };
            state.select(Some(i));
            self.update_panes();
        }
    }

    fn move_up(&mut self) {
        let state = match self.active_pane {
            Pane::Previous => &mut self.previous_state,
            Pane::Current => &mut self.current_state,
            Pane::Next => &mut self.next_state,
        };

        if let Some(i) = state.selected() {
            if i > 0 {
                state.select(Some(i - 1));
                self.update_panes();
            }
        }
    }

    fn move_left(&mut self) {
        if self.active_pane != Pane::Previous {
            self.active_pane = self.active_pane.previous();
        }
    }

    fn move_right(&mut self) {
        if self.active_pane == Pane::Current && self.current_state.selected().is_some() {
            if !self.next_items.is_empty() {
                self.active_pane = Pane::Next;
                if self.next_state.selected().is_none() {
                    self.next_state.select(Some(0));
                }
            }
        } else if self.active_pane == Pane::Previous && self.previous_state.selected().is_some() {
            self.active_pane = Pane::Current;
        }
    }

    fn update_panes(&mut self) {
        let selected_path = match self.active_pane {
            Pane::Previous => self
                .previous_state
                .selected()
                .and_then(|i| self.previous_items.get(i))
                .cloned(),
            Pane::Current => self
                .current_state
                .selected()
                .and_then(|i| self.current_items.get(i))
                .cloned(),
            Pane::Next => self
                .next_state
                .selected()
                .and_then(|i| self.next_items.get(i))
                .cloned(),
        };

        if let Some(path) = selected_path {
            self.current_path = Some(path.clone());

            if self.active_pane == Pane::Current {
                self.previous_items = self
                    .stats
                    .get(&path)
                    .map(|s| s.immediate_parents.clone())
                    .unwrap_or_default();
                crate::path_stats::sort_paths(
                    &mut self.previous_items,
                    &self.stats,
                    self.sort_order,
                );

                let mut refs = self
                    .graph
                    .get_references(&path)
                    .into_iter()
                    .map(|p| p.path.clone())
                    .collect::<Vec<_>>();
                crate::path_stats::sort_paths(&mut refs, &self.stats, self.sort_order);
                self.next_items = refs;

                self.previous_state = ListState::default();
                self.next_state = ListState::default();
            }
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
}
