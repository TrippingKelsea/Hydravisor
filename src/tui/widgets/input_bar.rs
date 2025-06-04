// src/tui/widgets/input_bar.rs
use ratatui::{
    layout::Rect,
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use crate::tui::{App, InputMode, TuiView};

pub struct InputBarWidget;

impl InputBarWidget {
    pub fn render(f: &mut Frame, app: &App, area: Rect) {
        let theme = &app.theme;

        let title_string;
        let input_style;

        if app.input_mode == InputMode::Editing {
            let model_name = app.active_chat.as_ref().map_or("Chat", |ac| &ac.model_name);
            title_string = format!("Input to {}: (Esc to nav, Enter to send)", model_name);
            input_style = Style::default().fg(theme.input_bar_text_fg).bg(theme.input_bar_background);
        } else {
            title_string = "Press 'i' to input (Chat View), <Tab>/<S-Tab> to switch views, 'q' to quit".to_string();
            input_style = Style::default().fg(theme.secondary_foreground).bg(theme.input_bar_background);
        }

        let current_text_display_string = if app.input_mode == InputMode::Editing {
            format!("{}>", app.current_input)
        } else {
            if app.active_view != TuiView::Chat {
                "(Input not active for this view)".to_string()
            } else {
                app.current_input.clone()
            }
        };
        
        let paragraph_text = Text::from(current_text_display_string);

        let input_block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.input_bar_border))
            .title(Line::from(Span::styled(title_string, Style::default().fg(theme.primary_foreground).bg(theme.input_bar_background))))
            .style(Style::default().bg(theme.input_bar_background));

        let input_area_widget = Paragraph::new(paragraph_text)
            .style(input_style)
            .block(input_block);
            
        f.render_widget(input_area_widget, area);
    }
}
