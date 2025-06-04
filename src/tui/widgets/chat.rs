// src/tui/widgets/chat.rs
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use crate::tui::App;
use unicode_width;

pub struct ChatWidget;

// Helper function for simple character-based wrapping (UTF-8 aware)
fn wrap_text(text: &str, width: u16) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }
    let mut lines = Vec::new();
    let mut current_line = String::new();
    let mut current_width = 0;
    for c in text.chars() {
        let char_width = unicode_width::UnicodeWidthChar::width(c).unwrap_or(1) as u16;
        if current_width + char_width > width {
            lines.push(current_line.clone());
            current_line.clear();
            current_width = 0;
        }
        current_line.push(c);
        current_width += char_width;
    }
    if !current_line.is_empty() {
        lines.push(current_line);
    }
    if lines.is_empty() && !text.is_empty() { // Handle case where text is shorter than width or width is very small
        lines.push(text.to_string());
    }
    lines
}

impl ChatWidget {
    // app needs to be mutable for chat_list_state
    pub fn render(f: &mut Frame, app: &mut App, area: Rect) { 
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30), // Left pane for chat info - decreased size
                Constraint::Percentage(70), // Right pane for messages - increased size
            ].as_ref())
            .split(area);

        // Left Pane: Chat Info
        let left_pane_block = Block::default().title("Chat Info").borders(Borders::ALL);
        let left_pane_content_area = left_pane_block.inner(chunks[0]);
        f.render_widget(left_pane_block, chunks[0]);

        let chat_info_text = if let Some(chat_session) = &app.active_chat {
            format!(
                "Model: {}\nMessages: {}\nStreaming: {}",
                chat_session.model_name,
                chat_session.messages.len(),
                if chat_session.is_streaming { "Yes" } else { "No" }
            )
        } else {
            "No active chat. Select model from Ollama list and press <Enter>.".to_string()
        };
        f.render_widget(Paragraph::new(Text::from(chat_info_text)).wrap(ratatui::widgets::Wrap { trim: true }), left_pane_content_area);

        // Right Pane: Chat Messages
        let right_pane_title = if let Some(chat) = &app.active_chat {
            format!("Chat with {} ({})", chat.model_name, if chat.is_streaming {"streaming..."} else {"idle"})
        } else {
            "Chat Area".to_string()
        };
        let right_pane_block = Block::default().title(Line::from(right_pane_title)).borders(Borders::ALL);
        let messages_area = right_pane_block.inner(chunks[1]);
        f.render_widget(right_pane_block, chunks[1]);

        // Calculate available width for message content (subtracting a bit for padding/sender name if needed)
        // For simplicity, using messages_area.width directly for wrapping text content.
        // A more precise calculation might subtract space for sender, timestamp, list markers, etc.
        let content_width = messages_area.width.saturating_sub(4); // Approx: 2 for borders/padding, 2 for list marker + space

        if let Some(chat_session) = &mut app.active_chat { // Changed to &mut for chat_list_state later
            let message_items: Vec<ListItem> = chat_session.messages.iter().enumerate().map(|(idx, msg)| {
                let sender_style = if msg.sender == "user" {
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                };
                
                let mut current_content_str = msg.content.clone();
                if chat_session.is_streaming && idx == chat_session.messages.len() - 1 && msg.sender != "user" {
                    current_content_str.push_str("...");
                }

                let wrapped_content_lines: Vec<Line> = wrap_text(&current_content_str, content_width)
                    .into_iter()
                    .map(Line::from)
                    .collect();

                let mut lines_for_list_item = vec![
                    Line::from(Span::styled(format!("{}: ", msg.sender), sender_style)),
                    Line::from(Span::styled(format!("  (@{})", msg.timestamp), Style::default().fg(Color::DarkGray))).alignment(ratatui::layout::Alignment::Right),
                    Line::from(""), 
                ];
                lines_for_list_item.extend(wrapped_content_lines);
                lines_for_list_item.push(Line::from("")); 

                ListItem::new(Text::from(lines_for_list_item))
            }).collect();
            
            let chat_list = List::new(message_items)
                .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
                .highlight_symbol("> "); // Optional

            f.render_stateful_widget(chat_list, messages_area, &mut app.chat_list_state);
        } else {
            f.render_widget(Paragraph::new("No active chat. Select a model from the Ollama list and press Enter."), messages_area);
        }
    }
}
