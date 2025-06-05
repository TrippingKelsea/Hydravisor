// src/tui/widgets/input_bar.rs
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use crate::tui::{App, InputMode};

pub struct InputBarWidget;

impl InputBarWidget {
    pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
        let theme = &app.theme;

        let current_input_text = &app.current_input;
        let input_field_text = if app.input_mode == InputMode::Editing {
            format!("{}{}", current_input_text, "▋") // Use a block cursor
        } else {
            current_input_text.to_string()
        };

        let title = if let Some(model_name) = &app.editing_system_prompt_for_model {
            Line::from(vec![
                Span::styled("Editing System Prompt for ", theme.input_bar_title),
                Span::styled(model_name.clone(), theme.input_bar_title.patch(Style::default().add_modifier(Modifier::BOLD))),
                Span::styled(":", theme.input_bar_title),
            ])
        } else if app.active_view == crate::tui::TuiView::Chat && app.active_chat.is_some() {
            Line::from(Span::styled("Chat Input:", theme.input_bar_title))
        } else {
            Line::from(Span::styled("Input:", theme.input_bar_title)) // Generic fallback
        };
        
        let input_block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(if app.input_mode == InputMode::Editing {
                theme.border_accent // Highlight border when editing
            } else {
                theme.border_secondary
            }));

        let paragraph = Paragraph::new(input_field_text)
            .block(input_block)
            .style(Style::default().fg(theme.input_bar_text));
        
        f.render_widget(paragraph, area);
    }
}
