use ratatui::{
    layout::Rect,
    style::{Color, Style, Modifier},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use tracing::Level; // For matching log levels

use crate::tui::App;

pub struct LogsWidget;

impl LogsWidget {
    pub fn render(f: &mut Frame, app: &mut App, area: Rect) { // app needs to be mutable for list_state
        let log_items: Vec<ListItem> = app.log_entries.iter().map(|log_entry| {
            let level_style = match log_entry.level {
                Level::ERROR => Style::default().fg(Color::Red),
                Level::WARN => Style::default().fg(Color::Yellow),
                Level::INFO => Style::default().fg(Color::Cyan),
                Level::DEBUG => Style::default().fg(Color::Green),
                Level::TRACE => Style::default().fg(Color::Magenta),
            };

            let timestamp_span = Span::styled(
                format!("{} ", log_entry.timestamp),
                Style::default().fg(Color::DarkGray),
            );
            let level_span = Span::styled(
                format!("{:<5} ", log_entry.level.as_str()), // as_str() requires tracing::Level to be in scope
                level_style.add_modifier(Modifier::BOLD),
            );
            let target_span = Span::styled(
                format!("[{}] ", log_entry.target),
                Style::default().fg(Color::Gray).add_modifier(Modifier::ITALIC),
            );
            let message_span = Span::raw(log_entry.message.clone());

            let line = Line::from(vec![timestamp_span, level_span, target_span, message_span]);
            ListItem::new(line)
        }).collect();

        if log_items.is_empty() {
            let placeholder = Paragraph::new("No log entries yet.")
                .block(Block::default().title("Logs").borders(Borders::ALL))
                .style(Style::default().fg(Color::DarkGray));
            f.render_widget(placeholder, area);
        } else {
            let log_list = List::new(log_items)
                .block(Block::default().title("Logs").borders(Borders::ALL))
                .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
                .highlight_symbol("> "); // Optional: symbol for selected item

            // Pass the mutable state to render_stateful_widget
            f.render_stateful_widget(log_list, area, &mut app.log_list_state);
        }
    }
}
