// src/tui/widgets/input_bar.rs
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use crate::tui::{App, InputMode};
use textwrap;

pub struct InputBarWidget;

const CURSOR_CHAR: &str = "▋";

impl InputBarWidget {
    pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
        let theme = &app.theme;

        let title = if let Some(model_name) = &app.editing_system_prompt_for_model {
            Line::from(vec![
                Span::styled("Editing System Prompt for ", theme.input_bar_title),
                Span::styled(model_name.clone(), theme.input_bar_title.patch(Style::default().add_modifier(Modifier::BOLD))),
                Span::styled(":", theme.input_bar_title),
            ])
        } else if app.active_view == crate::tui::TuiView::Chat && app.active_chat.is_some() && app.input_mode == InputMode::Editing {
            Line::from(Span::styled("Chat Input (Esc to cancel/send with Enter):", theme.input_bar_title))
        } else {
            Line::from(Span::styled("Input:", theme.input_bar_title)) // Generic fallback or for normal mode
        };
        
        let input_block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(if app.input_mode == InputMode::Editing || app.editing_system_prompt_for_model.is_some() {
                theme.border_accent // Highlight border when actively editing (chat or system prompt)
            } else {
                theme.border_secondary
            }));
        
        // The actual area for text content inside the block
        let text_area_width = area.width.saturating_sub(2); // Subtract borders

        let text_to_display: Text = if app.input_mode == InputMode::Editing || app.editing_system_prompt_for_model.is_some() {
            let text_with_cursor = format!("{}{}", app.current_input, CURSOR_CHAR);
            let wrapped_lines: Vec<Line> = textwrap::wrap(&text_with_cursor, text_area_width as usize)
                .iter()
                .map(|s| Line::from(s.to_string()))
                .collect();
            Text::from(wrapped_lines)
        } else if !app.current_input.is_empty(){
            // Display a placeholder or truncated version if not in edit mode but there's text (e.g. after sending)
            // For now, let's just show it wrapped if it exists (e.g. for system prompt preview when not editing)
            let wrapped_lines: Vec<Line> = textwrap::wrap(&app.current_input, text_area_width as usize)
                .iter()
                .map(|s| Line::from(s.to_string()))
                .collect();
            Text::from(wrapped_lines)
        } else {
            Text::from("") // Empty if not editing and no text
        };

        let paragraph = Paragraph::new(text_to_display)
            .block(input_block)
            .style(Style::default().fg(theme.input_bar_text));
        
        f.render_widget(paragraph, area);
    }
}
