use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Style, Modifier},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use crate::tui::App;

pub struct KeybindingsModalWidget;

impl KeybindingsModalWidget {
    pub fn render(f: &mut Frame, app: &App, area: Rect) {
        let theme = &app.theme;
        let popup_area = Rect {
            x: area.x + area.width / 4,
            y: area.y + area.height / 4,
            width: area.width / 2,
            height: area.height / 2,
        };
        f.render_widget(Clear, popup_area);
        let block = Block::default()
            .title("Keybindings")
            .borders(Borders::ALL)
            .style(Style::default().fg(theme.primary_foreground).bg(theme.primary_background))
            .title_alignment(Alignment::Center);
        f.render_widget(block.clone(), popup_area);
        let inner = block.inner(popup_area);
        let kb = &app.config.keybindings;
        let lines = vec![
            Line::from(vec![Span::styled("Quit: ", Style::default().add_modifier(Modifier::BOLD)), Span::raw(&kb.quit)]),
            Line::from(vec![Span::styled("Help: ", Style::default().add_modifier(Modifier::BOLD)), Span::raw(&kb.help)]),
            Line::from(vec![Span::styled("Menu: ", Style::default().add_modifier(Modifier::BOLD)), Span::raw(&kb.menu)]),
            Line::from(vec![Span::styled("Next Tab: ", Style::default().add_modifier(Modifier::BOLD)), Span::raw(&kb.next_tab)]),
            Line::from(vec![Span::styled("Prev Tab: ", Style::default().add_modifier(Modifier::BOLD)), Span::raw(&kb.prev_tab)]),
            Line::from(vec![Span::styled("New VM: ", Style::default().add_modifier(Modifier::BOLD)), Span::raw(&kb.new_vm)]),
            Line::from(vec![Span::styled("Destroy VM: ", Style::default().add_modifier(Modifier::BOLD)), Span::raw(&kb.destroy_vm)]),
            Line::from(vec![Span::styled("Edit: ", Style::default().add_modifier(Modifier::BOLD)), Span::raw(&kb.edit)]),
            Line::from(vec![Span::styled("Enter: ", Style::default().add_modifier(Modifier::BOLD)), Span::raw(&kb.enter)]),
            Line::from(vec![Span::styled("Up: ", Style::default().add_modifier(Modifier::BOLD)), Span::raw(&kb.up)]),
            Line::from(vec![Span::styled("Down: ", Style::default().add_modifier(Modifier::BOLD)), Span::raw(&kb.down)]),
            Line::from("")
        ];
        let mut lines = lines;
        lines.push(Line::from(vec![Span::styled("Press Esc to close", Style::default().fg(theme.help_text))]));
        let para = Paragraph::new(lines)
            .alignment(Alignment::Left)
            .block(Block::default());
        f.render_widget(para, inner);
    }
} 