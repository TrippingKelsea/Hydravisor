use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use chrono::Local;
use crate::tui::{App, InputMode};

pub struct StatusBarWidget;

impl StatusBarWidget {
    pub fn render(f: &mut Frame, app: &App, area: Rect) {
        let theme = &app.theme;

        let status_bar_style = Style::default()
            .fg(theme.status_bar_foreground)
            .bg(theme.status_bar_background);

        let status_bar_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(75),
                Constraint::Percentage(25),
            ])
            .split(area);

        let view_mode_bg = match app.input_mode {
            InputMode::Normal => theme.status_bar_mode_normal_bg,
            InputMode::Editing => theme.status_bar_mode_editing_bg,
            InputMode::VmWizard => theme.status_bar_mode_vm_wizard_bg,
            InputMode::ConfirmingDestroy => theme.status_bar_mode_confirm_destroy_bg,
        };

        let status_spans_left = Line::from(vec![
            Span::styled("Hydravisor | ", status_bar_style),
            Span::styled("View: ", status_bar_style),
            Span::styled(format!("{:?}", app.active_view), 
                         Style::default().fg(theme.status_bar_view_name_fg).bg(theme.status_bar_background).bold()),
            Span::styled(" | Input: ", status_bar_style),
            Span::styled(format!("{:?}", app.input_mode), 
                         Style::default().fg(theme.primary_foreground).bg(view_mode_bg).bold()),
            Span::styled(format!(" | VMs: {} ", app.vms.len()), status_bar_style),
            Span::styled(format!("| Ollama: {} ", 
                if cfg!(feature = "ollama_integration") { app.ollama_models.len().to_string() } else { "N/A".to_string() }), 
                status_bar_style),
        ]);
        
        f.render_widget(Paragraph::new(status_spans_left).style(status_bar_style), status_bar_layout[0]);

        let status_text_right = Local::now().format("%H:%M:%S").to_string();
        f.render_widget(
            Paragraph::new(status_text_right)
                .style(status_bar_style)
                .alignment(Alignment::Right),
            status_bar_layout[1]
        );
    }
}
