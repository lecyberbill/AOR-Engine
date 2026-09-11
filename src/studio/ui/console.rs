// [WFGY] Zone: SAFE | λ: 0.25 | Fallbacks: 0/None | Action: Console & Rhai interactive terminal panel
#![allow(dead_code)]
use super::style::{column, leaf};
use ui_layout::NodeId;
use ui_widgets::{ListItemBadge, WidgetId, WidgetTree};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Success,
    Warning,
    Error,
}

impl LogLevel {
    pub fn badge(&self) -> ListItemBadge {
        match self {
            LogLevel::Info => ListItemBadge::None,
            LogLevel::Success => ListItemBadge::Success,
            LogLevel::Warning => ListItemBadge::Warning,
            LogLevel::Error => ListItemBadge::Active("ERR".to_string()),
        }
    }

    pub fn prefix(&self) -> &'static str {
        match self {
            LogLevel::Info => "[Info]",
            LogLevel::Success => "[ OK ]",
            LogLevel::Warning => "[Warn]",
            LogLevel::Error => "[Err ]",
        }
    }
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub message: String,
    pub level: LogLevel,
}

#[derive(Debug, Clone)]
pub struct ConsoleState {
    pub logs: Vec<LogEntry>,
    pub input: String,
    pub history: Vec<String>,
    pub history_cursor: Option<usize>,
}

impl Default for ConsoleState {
    fn default() -> Self {
        ConsoleState {
            logs: vec![LogEntry {
                message: "Moteur AOR prêt. Tapez une commande Rhai.".to_string(),
                level: LogLevel::Info,
            }],
            input: String::new(),
            history: Vec::new(),
            history_cursor: None,
        }
    }
}

impl ConsoleState {
    pub fn push(&mut self, message: impl Into<String>, level: LogLevel) {
        self.logs.push(LogEntry {
            message: message.into(),
            level,
        });
    }

    /// Soumet la commande courante : l'enregistre dans l'historique et la journalise
    pub fn submit(&mut self) {
        let cmd = self.input.trim().to_string();
        if cmd.is_empty() {
            return;
        }
        self.history.push(cmd.clone());
        self.history_cursor = None;
        self.push(format!("> rhai: {}", cmd), LogLevel::Info);
        self.input.clear();
    }

    pub fn navigate_history(&mut self, backward: bool) {
        if self.history.is_empty() {
            return;
        }
        let len = self.history.len();
        let cursor = match (self.history_cursor, backward) {
            (None, true) => len - 1,
            (None, false) => 0,
            (Some(0), true) => 0,
            (Some(i), true) => i - 1,
            (Some(i), false) if i + 1 >= len => len - 1,
            (Some(i), false) => i + 1,
        };
        self.history_cursor = Some(cursor);
        self.input = self.history[cursor].clone();
    }
}

/// Construit le panneau console + terminal Rhai
pub fn build(
    tree: &mut WidgetTree,
    state: &ConsoleState,
    width: f32,
    height: f32,
) -> NodeId {
    let items: Vec<(String, ListItemBadge)> = state
        .logs
        .iter()
        .rev()
        .take(200)
        .map(|e| (format!("{} {}", e.level.prefix(), e.message), e.level.badge()))
        .collect();

    let inner_w = (width - 24.0).max(100.0);
    let list = tree
        .rich_list(
            WidgetId::new("console_logs"),
            &items,
            None,
            leaf(inner_w, 18.0),
            column(1.0),
        )
        .expect("console logs");

    let input = tree
        .text_area(
            WidgetId::new("rhai_terminal"),
            state.input.clone(),
            "> rhai: player.boost = 100.0;",
            true,
            false,
            leaf(inner_w, 44.0),
        )
        .expect("rhai terminal");

    let body = tree.container(&[list, input], column(4.0))
        .expect("console panel body");
    tree.palette(
        WidgetId::new("studio_console_panel"),
        "CONSOLE & TERMINAL RHAI",
        false,
        Some(body),
        leaf(width, height),
    )
    .expect("console panel palette")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_submit_adds_history_and_log() {
        let mut c = ConsoleState::default();
        c.input = "spawn_drone()".into();
        c.submit();
        assert_eq!(c.history.len(), 1);
        assert!(c.input.is_empty());
        assert!(c.logs.last().unwrap().message.contains("spawn_drone()"));
    }

    #[test]
    fn test_history_navigation() {
        let mut c = ConsoleState::default();
        c.input = "a".into();
        c.submit();
        c.input = "b".into();
        c.submit();
        c.navigate_history(true);
        assert_eq!(c.input, "b");
        c.navigate_history(true);
        assert_eq!(c.input, "a");
        c.navigate_history(false);
        assert_eq!(c.input, "b");
    }
}
