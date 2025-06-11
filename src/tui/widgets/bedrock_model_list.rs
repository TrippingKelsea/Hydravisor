#![cfg(feature = "bedrock_integration")]

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use crate::tui::App;

pub struct BedrockModelListWidget;

impl BedrockModelListWidget {
    pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
        let theme = &app.theme;

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(40),
                Constraint::Percentage(60),
            ].as_ref())
            .split(area);

        let left_pane_block = Block::default()
            .title(Line::from(Span::styled("Bedrock Models", Style::default().fg(theme.primary_foreground).bold())))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border_primary));
        let left_pane_content_area = left_pane_block.inner(chunks[0]);
        f.render_widget(left_pane_block, chunks[0]);

        let model_items: Vec<ListItem> = app.bedrock_models.iter()
            .map(|model| {
                let model_name = model.model_name().unwrap_or("Unknown Model");
                ListItem::new(Line::from(Span::styled(model_name.to_string(), Style::default().fg(theme.primary_foreground))))
            })
            .collect();

        let model_list = List::new(model_items)
            .highlight_style(Style::default().fg(theme.list_highlight_fg).bg(theme.list_highlight_bg))
            .highlight_symbol(">> ");

        f.render_stateful_widget(model_list, left_pane_content_area, &mut app.bedrock_model_list_state);

        let right_pane_block = Block::default()
            .title(Line::from(Span::styled("Model Details", Style::default().fg(theme.primary_foreground))))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border_secondary));
        let right_pane_content_area = right_pane_block.inner(chunks[1]);
        f.render_widget(right_pane_block.clone(), chunks[1]);

        if let Some(selected_idx) = app.bedrock_model_list_state.selected() {
            if let Some(model) = app.bedrock_models.get(selected_idx) {
                let mut details_lines = vec![
                    Line::from(vec![Span::styled("Name: ", theme.ollama_model_list_details_title.clone()), Span::raw(model.model_name().unwrap_or("N/A"))]),
                    Line::from(vec![Span::styled("ID: ", theme.ollama_model_list_details_title.clone()), Span::raw(model.model_id().unwrap_or("N/A"))]),
                    Line::from(vec![Span::styled("Provider: ", theme.ollama_model_list_details_title.clone()), Span::raw(model.provider_name().unwrap_or("N/A"))]),
                    Line::from(vec![Span::styled("Customizations: ", theme.ollama_model_list_details_title.clone()), Span::raw(format!("{:?}", model.customizations_supported()))]),
                    Line::from(vec![Span::styled("Inference Types: ", theme.ollama_model_list_details_title.clone()), Span::raw(format!("{:?}", model.inference_types_supported()))]),
                    Line::from(vec![Span::styled("Response Streaming: ", theme.ollama_model_list_details_title.clone()), Span::raw(format!("{}", model.response_streaming_supported().unwrap_or(false)))]),
                ];
                f.render_widget(Paragraph::new(Text::from(details_lines)).wrap(ratatui::widgets::Wrap { trim: false }).style(Style::default().fg(theme.primary_foreground)), right_pane_content_area);
            } else {
                f.render_widget(Paragraph::new("No model selected or data unavailable.").style(Style::default().fg(theme.secondary_foreground)), right_pane_content_area);
            }
        } else {
            f.render_widget(Paragraph::new("No model selected").style(Style::default().fg(theme.secondary_foreground)), right_pane_content_area);
        }
    }
} 