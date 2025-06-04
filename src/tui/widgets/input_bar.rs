// src/tui/widgets/input_bar.rs
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Text},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use crate::tui::{App, InputMode}; // Corrected path to App and InputMode

pub struct InputBarWidget;

impl InputBarWidget {
    pub fn render(f: &mut Frame, app: &App, area: Rect) {
        let input_block_title_string = if app.input_mode == InputMode::Editing {
            if app.active_chat.is_some() {
                format!("Input to {}: (Esc to nav, Enter to send)", app.active_chat.as_ref().unwrap().model_name)
            } else { "Input (Esc to nav, Enter to send)".to_string() }
        } else { "Press 'i' to input, <Tab> to switch views, 'q' to quit".to_string() };
        let title_line = Line::from(input_block_title_string);

        let current_text_display_string = if app.input_mode == InputMode::Editing {
            format!("{}{}", app.current_input, "_") // Show cursor in editing mode
        } else {
            app.current_input.clone()
        };
        let paragraph_text = Text::from(current_text_display_string);

        let input_area_widget = Paragraph::new(paragraph_text)
            .style(match app.input_mode {
                InputMode::Normal => Style::default().fg(Color::DarkGray),
                InputMode::Editing => Style::default().fg(Color::Yellow),
            })
            .block(Block::default().borders(Borders::ALL).title(title_line));
        f.render_widget(input_area_widget, area);
    }
}
