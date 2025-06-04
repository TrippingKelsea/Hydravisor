// src/tui/widgets/chat.rs
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use crate::tui::App;

pub struct ChatWidget;

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

                // Each line of the potentially multi-line content becomes an owned String, then a Line
                let content_lines: Vec<Line> = current_content_str
                    .lines()
                    .map(|s| Line::from(s.to_string())) // Convert to owned String for the Line
                    .collect();
                let message_text_widget = Text::from(content_lines);

                // Construct the list item content
                let mut lines_for_list_item = vec![
                    Line::from(Span::styled(format!("{}: ", msg.sender), sender_style)),
                    Line::from(Span::styled(format!("  (@{})", msg.timestamp), Style::default().fg(Color::DarkGray))).alignment(ratatui::layout::Alignment::Right),
                    Line::from(""), // Spacer line before content
                ];
                
                // Add all lines from the message_text_widget
                // Since message_text_widget.lines is Vec<Line<'a>>, and we need owned for ListItem, we ensure above conversion
                lines_for_list_item.extend(message_text_widget.lines); // Now this should be Vec<Line<'static>> effectively due to .to_string()
                lines_for_list_item.push(Line::from("")); // Spacer line after content

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
