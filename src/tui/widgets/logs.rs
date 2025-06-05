use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use tracing::Level; // For matching log levels

use crate::tui::App;

pub struct LogsWidget;

impl LogsWidget {
    pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
        let theme = &app.theme;

        let title_block = Block::default()
            .title(Line::from(Span::styled("Logs", theme.log_title)))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border_primary));

        if app.log_entries.is_empty() {
            let placeholder = Paragraph::new("No log entries yet.")
                .block(title_block)
                .style(Style::default().fg(theme.secondary_foreground));
            f.render_widget(placeholder, area);
        } else {
            let log_items: Vec<ListItem> = app.log_entries.iter().map(|log_entry| {
                let level_style = match log_entry.level {
                    Level::ERROR => theme.log_level_error.clone(),
                    Level::WARN => theme.log_level_warn.clone(),
                    Level::INFO => theme.log_level_info.clone(),
                    Level::DEBUG => theme.log_level_debug.clone(),
                    Level::TRACE => theme.log_level_trace.clone(),
                };

                let timestamp_span = Span::styled(
                    format!("{} ", log_entry.timestamp),
                    theme.log_timestamp.clone(),
                );
                let level_span = Span::styled(
                    format!("{:<5} ", log_entry.level.as_str()),
                    level_style,
                );
                let target_span = Span::styled(
                    format!("[{}] ", log_entry.target),
                    theme.log_target.clone(),
                );
                let message_span = Span::styled(log_entry.message.clone(), Style::default().fg(theme.primary_foreground));

                let line = Line::from(vec![timestamp_span, level_span, target_span, message_span]);
                ListItem::new(line)
            }).collect();

            let log_list = List::new(log_items)
                .block(title_block)
                .highlight_style(theme.highlight_style.clone())
                .highlight_symbol("> ");

            f.render_stateful_widget(log_list, area, &mut app.log_list_state);
        }
    }
}
