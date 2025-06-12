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
        // Menu structure: Main -> Preferences -> Keybindings
        let main_items = [
            ListItem::new(Line::from(vec![Span::raw("About")])),
            ListItem::new(Line::from(vec![Span::raw("Preferences")])),
            ListItem::new(Line::from(vec![Span::raw("Quit")])),
        ];
        let prefs_items = [
            ListItem::new(Line::from(vec![Span::raw("Keybindings")])),
        ];

        let (items, title, menu_height) = match app.menu_level {
            0 => (&main_items[..], "Menu", 6),
            1 => (&prefs_items[..], "Preferences", 4),
            _ => (&main_items[..], "Menu", 6),
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .style(Style::default().fg(theme.primary_foreground).bg(theme.primary_background))
            )
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("> ");

        // Simple positioning for now, can be improved
        let menu_area = Rect {
            x: area.x + 5,
            y: area.y + 1,
            width: 24,
            height: menu_height,
        };

        f.render_widget(Clear, menu_area); //this clears the background
        if app.menu_level == 0 {
            f.render_stateful_widget(list, menu_area, &mut app.menu_state);
        } else if app.menu_level == 1 {
            f.render_stateful_widget(list, menu_area, &mut app.menu_sub_state);
        }
    }
} 