use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Text,
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use crate::tui::App;


pub struct OllamaModelListWidget;

impl OllamaModelListWidget {
    pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(40),
                Constraint::Percentage(60),
            ].as_ref())
            .split(area);

        // Left Pane: Ollama Model List
        let left_pane_block = Block::default().title("Ollama Models").borders(Borders::ALL);
        let left_pane_content_area = left_pane_block.inner(chunks[0]);
        f.render_widget(left_pane_block, chunks[0]);

        #[cfg(feature = "ollama_integration")] {
            let model_items: Vec<ListItem> = app.ollama_models.iter()
                .map(|model| ListItem::new(model.name.clone()))
                .collect();
            let model_list = List::new(model_items)
                .highlight_style(Style::default().add_modifier(Modifier::BOLD).bg(Color::Gray))
                .highlight_symbol(">> ");
            f.render_stateful_widget(model_list, left_pane_content_area, &mut app.ollama_model_list_state);
        }
        #[cfg(not(feature = "ollama_integration"))] {
            let placeholder_items: Vec<ListItem> = app.ollama_models.iter()
                .map(|s| ListItem::new(s.as_str()))
                .collect();
            f.render_widget(List::new(placeholder_items), left_pane_content_area);
        }

        // Right Pane: Model Details
        let right_pane_block = Block::default().title("Model Details").borders(Borders::ALL);
        let right_pane_content_area = right_pane_block.inner(chunks[1]);
        f.render_widget(right_pane_block, chunks[1]);

        #[cfg(feature = "ollama_integration")] {
            if let Some(selected_idx) = app.ollama_model_list_state.selected() {
                if let Some(model) = app.ollama_models.get(selected_idx) {
                    let details = format!("Name: {}\nModified: {}\nSize: {}", model.name, model.modified_at, model.size);
                    f.render_widget(Paragraph::new(Text::from(details)), right_pane_content_area);
                } else {
                    f.render_widget(Paragraph::new("No model selected or data unavailable."), right_pane_content_area);
                }
            } else {
                f.render_widget(Paragraph::new("No model selected"), right_pane_content_area);
            }
        }
        #[cfg(not(feature = "ollama_integration"))] {
            f.render_widget(Paragraph::new("Ollama N/A. No model details."), right_pane_content_area);
        }
    }
}
