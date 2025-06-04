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
    pub fn render(f: &mut Frame, app: &App, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(40), // Left pane for chat info
                Constraint::Percentage(60), // Right pane for messages
            ].as_ref())
            .split(area);

        // Left Pane: Chat Info (placeholder)
        let left_pane_block = Block::default().title("Chat Info").borders(Borders::ALL);
        let left_pane_content_area = left_pane_block.inner(chunks[0]);
        f.render_widget(left_pane_block, chunks[0]);

        let chat_info_text = if let Some(chat_session) = &app.active_chat {
            format!("Chatting with: {}\n(Details in right pane)", chat_session.model_name)
        } else {
            "No active chat. Select model & <Enter>.".to_string()
        };
        f.render_widget(Paragraph::new(Text::from(chat_info_text)), left_pane_content_area);

        // Right Pane: Chat Messages
        let right_pane_title = if let Some(chat) = &app.active_chat {
            format!("Chat with {}", chat.model_name)
        } else {
            "Chat Area".to_string()
        };
        let right_pane_block = Block::default().title(Line::from(right_pane_title)).borders(Borders::ALL);
        let right_pane_content_area = right_pane_block.inner(chunks[1]);
        f.render_widget(right_pane_block, chunks[1]);

        if let Some(chat_session) = &app.active_chat {
            let messages: Vec<ListItem> = chat_session.messages.iter().map(|msg| {
                ListItem::new(Line::from(vec![
                    Span::styled(format!("[{}] ", msg.timestamp), Style::default().fg(Color::DarkGray)),
                    Span::styled(format!("{}: ", msg.sender), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                    Span::raw(&msg.content),
                ]))
            }).collect();
            f.render_widget(List::new(messages).block(Block::default().borders(Borders::NONE)), right_pane_content_area);
        } else {
            f.render_widget(Paragraph::new("No active chat. Select model first."), right_pane_content_area);
        }
    }
}
