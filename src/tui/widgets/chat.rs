// src/tui/widgets/chat.rs
// use chrono::Local; // Removed unused import
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect}, // Removed Alignment
    style::{Color, Style, Stylize},
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
        let theme = &app.theme;

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(40), // Left pane for chat info - increased to 40%
                Constraint::Percentage(60), // Right pane for messages - decreased to 60%
            ].as_ref())
            .split(area);

        // Left Pane: Chat Info
        let left_pane_block = Block::default()
            .title(Line::from(Span::styled("Chat Info", Style::default().fg(theme.primary_foreground))))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border_primary));
        let left_pane_content_area = left_pane_block.inner(chunks[0]);
        f.render_widget(left_pane_block, chunks[0]);

        let chat_info_display_text = if let Some(chat_session) = &app.active_chat {
            let info_lines = vec![
                Line::from(vec![Span::styled("Model: ", Style::default().fg(theme.secondary_foreground)), Span::styled(&chat_session.model_name, Style::default().fg(theme.chat_info_text).bold())]),
                Line::from(vec![Span::styled("Messages: ", Style::default().fg(theme.secondary_foreground)), Span::styled(chat_session.messages.len().to_string(), Style::default().fg(theme.chat_info_text))]),
                Line::from(vec![Span::styled("Streaming: ", Style::default().fg(theme.secondary_foreground)), Span::styled(if chat_session.is_streaming { "Yes" } else { "No" }, Style::default().fg(theme.chat_info_text))]),
            ];
            Text::from(info_lines)
        } else {
            Text::from(Line::from(Span::styled("No active chat. Select model and press <Enter>.", Style::default().fg(theme.secondary_foreground))))
        };
        f.render_widget(Paragraph::new(chat_info_display_text).wrap(ratatui::widgets::Wrap { trim: true }), left_pane_content_area);

        // Right Pane: Chat Messages
        let right_pane_title_str = if let Some(chat) = &app.active_chat {
            format!("Chat with {} ({})", chat.model_name, if chat.is_streaming {"streaming..."} else {"idle"})
        } else {
            "Chat Area".to_string()
        };
        let right_pane_block = Block::default()
            .title(Line::from(Span::styled(right_pane_title_str, theme.chat_title)))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border_secondary));
        let messages_area = right_pane_block.inner(chunks[1]);
        f.render_widget(right_pane_block, chunks[1]);

        let content_width = messages_area.width.saturating_sub(2) as usize;

        if let Some(chat_session) = &mut app.active_chat {
            let message_items: Vec<ListItem> = chat_session.messages.iter().enumerate().map(|(idx, msg)| {
                let sender_style = if msg.sender == "user" {
                    theme.chat_user_sender.clone()
                } else {
                    theme.chat_model_sender.clone()
                };
                
                let available_width_for_ts_line = messages_area.width; // Total width available for the line
                let timestamp_len = msg.timestamp.chars().count();
                
                // Effective width for text rendering seems to be 3 chars less than total available
                let effective_text_width = (available_width_for_ts_line as usize).saturating_sub(3);
                
                let num_spaces = if timestamp_len < effective_text_width {
                    effective_text_width - timestamp_len
                } else {
                    0 // Timestamp itself is too long or exactly fits the (reduced) effective width
                };
                
                let padding = " ".repeat(num_spaces);
                let formatted_timestamp_str = format!("{}{}", padding, msg.timestamp);

                let mut lines_for_list_item = vec![
                    Line::from(Span::styled(format!("{}: ", msg.sender), sender_style)),
                    Line::from(Span::styled(formatted_timestamp_str, theme.chat_timestamp.clone())), 
                ];

                // Render thought if present
                if let Some(thought_text) = &msg.thought {
                    if !thought_text.is_empty() {
                        lines_for_list_item.push(Line::from("")); // Add a blank line before thought
                        let wrapped_thought: Vec<Line> = textwrap::fill(thought_text, content_width)
                            .lines()
                            .map(|line_str| Line::from(Span::styled(line_str.to_string(), theme.chat_thought_style.clone())))
                            .collect();
                        lines_for_list_item.extend(wrapped_thought);
                    }
                }
                
                // Render main content
                let mut current_content_str = msg.content.clone();
                if chat_session.is_streaming && idx == chat_session.messages.len() - 1 && msg.sender != "user" {
                    current_content_str.push_str(Span::styled("...", Style::default().fg(theme.chat_streaming_indicator)).content.as_ref());
                }

                if !current_content_str.is_empty() {
                     if msg.thought.is_some() && !msg.thought.as_ref().unwrap_or(&String::new()).is_empty() {
                        lines_for_list_item.push(Line::from("")); // Add a blank line after thought, before content
                    } else if msg.thought.is_none() { // Only add this blank line if there was no thought at all
                        lines_for_list_item.push(Line::from("")); // Blank line after sender/timestamp, before content
                    }
                    
                    // ---- START REVERT DIAGNOSTIC FOR TEXT ----
                    let final_text_style = if msg.sender != "user" {
                        let mut model_text_style = theme.chat_model_content_style.clone();
                        if !theme.chat_model_content_use_background {
                            model_text_style = model_text_style.bg(Color::Reset);
                        }
                        model_text_style
                    } else {
                        Style::default().fg(theme.primary_foreground)
                    };
                    // ---- END REVERT DIAGNOSTIC FOR TEXT ----

                    let wrapped_content_lines: Vec<Line> = textwrap::fill(&current_content_str, content_width)
                        .lines()
                        .map(|line_str| Line::from(Span::styled(line_str.to_string(), final_text_style)))
                        .collect();
                    lines_for_list_item.extend(wrapped_content_lines);
                } else if current_content_str.is_empty() && msg.thought.is_some() && !msg.thought.as_ref().unwrap_or(&String::new()).is_empty() {
                    // If only thought exists and content is empty (e.g. after extraction)
                    // lines_for_list_item.push(Line::from(Span::styled("(Thought processed, no further output)", Style::default().fg(Color::DarkGray))));
                }


                lines_for_list_item.push(Line::from("")); // Blank line after each message block

                ListItem::new(Text::from(lines_for_list_item))
            }).collect();
            
            // ---- START REVERT DIAGNOSTIC FOR LIST ----
            let chat_list = List::new(message_items)
                .style(Style::default()) // Revert list background to default/transparent
                .highlight_style(theme.highlight_style.clone()) // Restore theme highlight style
            // ---- END REVERT DIAGNOSTIC FOR LIST ----
                .highlight_symbol("> ");

            f.render_stateful_widget(chat_list, messages_area, &mut app.chat_list_state);
        } else {
            f.render_widget(Paragraph::new("No active chat. Select a model and press Enter.").style(Style::default().fg(theme.secondary_foreground)).wrap(ratatui::widgets::Wrap { trim: true }), messages_area);
        }
    }
}
