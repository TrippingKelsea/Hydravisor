use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use chrono::Local;
use crate::tui::app::{App, InputMode, AppView};

pub struct StatusBarWidget;

impl StatusBarWidget {
    pub fn render(f: &mut Frame, app: &App, area: Rect) {
        let theme = &app.theme;

        let status_bar_style = Style::default()
            .fg(theme.status_bar_foreground)
            .bg(theme.status_bar_background);
        
        let outlined_h_style = if app.show_menu {
            Style::default().fg(theme.quaternary_foreground).bg(theme.primary_background).bold()
        } else {
            status_bar_style
        };

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
            Span::styled("H", outlined_h_style),
            Span::styled("ydravisor | ", status_bar_style),
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

        let mut status_spans_right = vec![];
        if app.active_view == AppView::VmList {
            let (status_text, status_style) = if app.libvirt_connected {
                ("Connected", Style::default().fg(theme.success_text))
            } else {
                ("Disconnected", Style::default().fg(theme.error_text))
            };
            status_spans_right.push(Span::styled("Libvirt: ", status_bar_style));
            status_spans_right.push(Span::styled(status_text, status_style));
            status_spans_right.push(Span::raw(" | "));
        }
        
        status_spans_right.push(Span::from(Local::now().format("%H:%M:%S").to_string()));

        f.render_widget(
            Paragraph::new(Line::from(status_spans_right))
                .style(status_bar_style)
                .alignment(Alignment::Right),
            status_bar_layout[1]
        );
    }
}
