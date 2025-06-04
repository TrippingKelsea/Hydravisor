use ratatui::{
    layout::Rect,
    style::{Color, Style}, // Modifier might be needed if styles use it.
    Frame,
};
use tui_logger::TuiLoggerWidget;
use crate::tui::App;

pub struct LogsWidget;

impl LogsWidget {
    pub fn render(f: &mut Frame, app: &App, area: Rect) {
        let log_widget = TuiLoggerWidget::default()
            .style_error(Style::default().fg(Color::Red))
            .style_warn(Style::default().fg(Color::Yellow))
            .style_info(Style::default().fg(Color::Cyan))
            .style_debug(Style::default().fg(Color::Green))
            .style_trace(Style::default().fg(Color::Magenta))
            .output_separator(':')
            .output_timestamp(Some("%H:%M:%S%.3N".to_string()))
            .output_target(true)
            .output_file(true)
            .output_line(true)
            .state(&app.log_widget_state);
            // Note: TuiLoggerWidget does not have .target_width_percentage()
            // or complex internal layout adjustments like TuiLoggerSmartWidget.
            // It typically just displays the logs as a list.

        f.render_widget(log_widget, area);
    }
}
