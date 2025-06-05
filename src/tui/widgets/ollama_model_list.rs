use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use crate::tui::App;
use textwrap;

pub struct OllamaModelListWidget;

impl OllamaModelListWidget {
    pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
        let theme = &app.theme;

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(40),
                Constraint::Percentage(60),
            ].as_ref())
            .split(area);

        // Left Pane: Ollama Model List
        let left_pane_block = Block::default()
            .title(Line::from(Span::styled("Ollama Models", theme.ollama_list_title)))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border_primary));
        let left_pane_content_area = left_pane_block.inner(chunks[0]);
        f.render_widget(left_pane_block, chunks[0]);

        #[cfg(feature = "ollama_integration")] {
            let model_items: Vec<ListItem> = app.ollama_models.iter()
                .map(|model| ListItem::new(Line::from(Span::styled(model.name.clone(), Style::default().fg(theme.primary_foreground)))))
                .collect();
            let model_list = List::new(model_items)
                .highlight_style(theme.highlight_style.clone())
                .highlight_symbol(">> ");
            f.render_stateful_widget(model_list, left_pane_content_area, &mut app.ollama_model_list_state);
        }
        #[cfg(not(feature = "ollama_integration"))] {
            let placeholder_items: Vec<ListItem> = app.ollama_models.iter()
                .map(|s| ListItem::new(Line::from(Span::styled(s.as_str(), Style::default().fg(theme.secondary_foreground)))))
                .collect();
            f.render_widget(List::new(placeholder_items).block(Block::default().style(Style::default().fg(theme.secondary_foreground))), left_pane_content_area);
        }

        // Right Pane: Model Details
        let right_pane_block = Block::default()
            .title(Line::from(Span::styled("Model Details", Style::default().fg(theme.primary_foreground))))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border_secondary));
        let right_pane_content_area = right_pane_block.inner(chunks[1]);
        f.render_widget(right_pane_block.clone(), chunks[1]);

        #[cfg(feature = "ollama_integration")] {
            if let Some(selected_idx) = app.ollama_model_list_state.selected() {
                if let Some(model) = app.ollama_models.get(selected_idx) {
                    let mut details_lines = vec![
                        Line::from(vec![Span::styled("Name: ", theme.ollama_model_details_label), Span::raw(&model.name)]),
                        Line::from(vec![Span::styled("Modified: ", theme.ollama_model_details_label), Span::raw(&model.modified_at)]),
                        Line::from(vec![Span::styled("Size: ", theme.ollama_model_details_label), Span::raw(format!("{}", model.size))]),
                        Line::from(""),
                    ];

                    // Simplified logic for the tag:
                    let active_system_prompt_tag_str = if app.editable_ollama_model_prompts.contains_key(&model.name) {
                        "(Model specific)".to_string()
                    } else {
                        "(Using default)".to_string()
                    };

                    let system_prompt_to_display = app.get_active_system_prompt(&model.name);
                    
                    details_lines.push(Line::from(vec![
                        Span::styled("System Prompt ", theme.ollama_system_prompt_label),
                        Span::styled(active_system_prompt_tag_str, theme.ollama_system_prompt_tag),
                    ]));
                    
                    let prompt_content_width = right_pane_content_area.width.saturating_sub(2) as usize;
                    textwrap::fill(&system_prompt_to_display, prompt_content_width)
                        .lines()
                        .for_each(|line_str| {
                            details_lines.push(Line::from(Span::styled(line_str.to_string(), theme.ollama_system_prompt_content)));
                        });
                    
                    f.render_widget(Paragraph::new(Text::from(details_lines)).wrap(ratatui::widgets::Wrap { trim: false }).style(Style::default().fg(theme.primary_foreground)), right_pane_content_area);
                } else {
                    f.render_widget(Paragraph::new("No model selected or data unavailable.").style(Style::default().fg(theme.secondary_foreground)), right_pane_content_area);
                }
            } else {
                f.render_widget(Paragraph::new("No model selected").style(Style::default().fg(theme.secondary_foreground)), right_pane_content_area);
            }
        }
        #[cfg(not(feature = "ollama_integration"))] {
            f.render_widget(Paragraph::new("Ollama N/A. No model details.").style(Style::default().fg(theme.secondary_foreground)), right_pane_content_area);
        }
    }
}
