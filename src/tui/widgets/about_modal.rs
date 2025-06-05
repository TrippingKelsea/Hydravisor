use ratatui::{
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Style},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use crate::tui::App;

pub struct AboutModalWidget;

impl AboutModalWidget {
    pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
        if !app.show_about_modal {
            return;
        }

        let theme = &app.theme;
        let block = Block::default()
            .title("About Hydravisor")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border_accent))
            .style(Style::default().bg(theme.popup_background));
        
        let popup_area = centered_rect(60, 50, area);
        f.render_widget(Clear, popup_area); //this clears the background
        f.render_widget(block.clone(), popup_area);

        let about_text = app.readme_content.clone();

        let paragraph = Paragraph::new(about_text)
            .style(Style::default().fg(theme.primary_foreground))
            .block(Block::default().borders(Borders::NONE))
            .alignment(Alignment::Left);
        
        let inner_area = block.inner(popup_area);
        f.render_widget(paragraph, inner_area);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ].as_ref())
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ].as_ref())
        .split(popup_layout[1])[1]
} 