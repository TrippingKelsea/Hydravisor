use ratatui::{
    layout::{Rect},
    style::{Style, Modifier},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem},
    Frame,
};
use crate::tui::App;

pub struct MenuWidget;

impl MenuWidget {
    pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
        if !app.show_menu {
            return;
        }

        let theme = &app.theme;
        let items = [
            ListItem::new(Line::from(vec![Span::raw("About")])),
            ListItem::new(Line::from(vec![Span::raw("Quit")])),
        ];

        let list = List::new(items)
            .block(
                Block::default()
                    .title("Menu")
                    .borders(Borders::ALL)
                    .style(Style::default().fg(theme.primary_foreground).bg(theme.primary_background))
            )
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("> ");

        // Simple positioning for now, can be improved
        let menu_area = Rect {
            x: area.x + 5,
            y: area.y + 1,
            width: 20,
            height: 4,
        };

        f.render_widget(Clear, menu_area); //this clears the background
        f.render_stateful_widget(list, menu_area, &mut app.menu_state);
    }
} 