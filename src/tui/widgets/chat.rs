// src/tui/widgets/chat.rs
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use crate::tui::App;
use textwrap;

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

        let content_width = messages_area.width.saturating_sub(2) as usize; // Adjusted for textwrap, ensure usize

        if let Some(chat_session) = &mut app.active_chat {
            let message_items: Vec<ListItem> = chat_session.messages.iter().enumerate().map(|(idx, msg)| {
                let sender_style = if msg.sender == "user" {
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                };
                
                let mut lines_for_list_item = vec![
                    Line::from(Span::styled(format!("{}: ", msg.sender), sender_style)),
                    // Consider moving timestamp to be part of the first line or a dedicated small side area if layout allows
                    // For now, keeping it as a separate line, right-aligned.
                    Line::from(Span::styled(format!("(@{})", msg.timestamp), Style::default().fg(Color::DarkGray))).alignment(ratatui::layout::Alignment::Right),
                ];

                // Render thought if present
                if let Some(thought_text) = &msg.thought {
                    if !thought_text.is_empty() {
                        lines_for_list_item.push(Line::from("")); // Add a blank line before thought
                        let wrapped_thought: Vec<Line> = textwrap::fill(thought_text, content_width)
                            .lines()
                            .map(|line_str| Line::from(Span::styled(line_str.to_string(), Style::default().fg(Color::Yellow).add_modifier(Modifier::ITALIC))))
                            .collect();
                        lines_for_list_item.extend(wrapped_thought);
                    }
                }
                
                // Render main content
                let mut current_content_str = msg.content.clone();
                if chat_session.is_streaming && idx == chat_session.messages.len() - 1 && msg.sender != "user" {
                    current_content_str.push_str("...");
                }

                if !current_content_str.is_empty() {
                     if msg.thought.is_some() && !msg.thought.as_ref().unwrap_or(&String::new()).is_empty() {
                        lines_for_list_item.push(Line::from("")); // Add a blank line after thought, before content
                    } else if msg.thought.is_none() { // Only add this blank line if there was no thought at all
                        lines_for_list_item.push(Line::from("")); // Blank line after sender/timestamp, before content
                    }
                    
                    let wrapped_content_lines: Vec<Line> = textwrap::fill(&current_content_str, content_width)
                        .lines()
                        .map(|line_str| Line::from(line_str.to_string())) // Default style for content
                        .collect();
                    lines_for_list_item.extend(wrapped_content_lines);
                } else if current_content_str.is_empty() && msg.thought.is_some() && !msg.thought.as_ref().unwrap_or(&String::new()).is_empty() {
                    // If only thought exists and content is empty (e.g. after extraction)
                    // lines_for_list_item.push(Line::from(Span::styled("(Thought processed, no further output)", Style::default().fg(Color::DarkGray))));
                }


                lines_for_list_item.push(Line::from("")); // Blank line after each message block

                ListItem::new(Text::from(lines_for_list_item))
            }).collect();
            
            let chat_list = List::new(message_items)
                .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
                .highlight_symbol("> ");

            f.render_stateful_widget(chat_list, messages_area, &mut app.chat_list_state);
        } else {
            f.render_widget(Paragraph::new("No active chat. Select a model from the Ollama list and press Enter.").wrap(ratatui::widgets::Wrap { trim: true }), messages_area);
        }
    }
}
