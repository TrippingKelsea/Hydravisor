use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    widgets::Paragraph,
    Frame,
};
use chrono::Local;
use crate::tui::App; // Corrected path to App

pub struct StatusBarWidget;

impl StatusBarWidget {
    pub fn render(f: &mut Frame, app: &App, area: Rect) {
        let status_bar_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(70), // For left text
                Constraint::Percentage(30), // For right text
            ])
            .split(area);

        let status_text_left = format!(
            "Hydravisor | View: {:?} | Input: {:?} | VMs: {} | Ollama: {}",
            app.active_view,
            app.input_mode,
            app.vms.len(),
            if cfg!(feature = "ollama_integration") { app.ollama_models.len().to_string() } else { "N/A".to_string() },
        );
        f.render_widget(Paragraph::new(status_text_left), status_bar_layout[0]);

        let status_text_right = Local::now().format("%H:%M:%S").to_string();
        f.render_widget(Paragraph::new(status_text_right).alignment(Alignment::Right), status_bar_layout[1]);
    }
}
