#![cfg(feature = "bedrock_integration")]

use aws_sdk_bedrock::types::FoundationModelLifecycleStatus;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use crate::tui::App;
use crate::tui::view_mode::list::{ListViewMode, ListFilter, ListSorter};
use std::rc::Rc;
use crate::config::BedrockFiltersConfig;

struct AvailableToUseFilter;
impl ListFilter<aws_sdk_bedrock::types::FoundationModelSummary> for AvailableToUseFilter {
    fn filter(&self, item: &aws_sdk_bedrock::types::FoundationModelSummary) -> bool {
        item.model_lifecycle()
            .map(|lc| lc.status())
            .map_or(false, |s| s == &FoundationModelLifecycleStatus::Active)
    }
}
struct AvailableToRequestAccessFilter;
impl ListFilter<aws_sdk_bedrock::types::FoundationModelSummary> for AvailableToRequestAccessFilter {
    fn filter(&self, item: &aws_sdk_bedrock::types::FoundationModelSummary) -> bool {
        item.model_lifecycle()
            .map(|lc| lc.status())
            .map_or(true, |s| s != &FoundationModelLifecycleStatus::Active)
    }
}
struct AlphabeticalSorter;
impl ListSorter<aws_sdk_bedrock::types::FoundationModelSummary> for AlphabeticalSorter {
    fn compare(&self, a: &aws_sdk_bedrock::types::FoundationModelSummary, b: &aws_sdk_bedrock::types::FoundationModelSummary) -> std::cmp::Ordering {
        a.model_name().unwrap_or("").cmp(b.model_name().unwrap_or(""))
    }
}

pub struct BedrockModelListWidget;

impl BedrockModelListWidget {
    pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
        let theme = &app.theme;
        // Set up filters and sorters dynamically
        app.bedrock_model_view_mode.filters.clear();
        match app.current_bedrock_filter.as_str() {
            "available_to_use" => app.bedrock_model_view_mode.add_filter(Rc::new(AvailableToUseFilter)),
            "available_to_request_access" => app.bedrock_model_view_mode.add_filter(Rc::new(AvailableToRequestAccessFilter)),
            _ => {},
        }
        app.bedrock_model_view_mode.sorters.clear();
        match app.current_bedrock_sort.as_str() {
            "alphabetical" => app.bedrock_model_view_mode.add_sorter(Rc::new(AlphabeticalSorter)),
            _ => {},
        }
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(40),
                Constraint::Percentage(60),
            ].as_ref())
            .split(area);
        let filter_label = format!("Filter: {}", app.current_bedrock_filter);
        let sort_label = format!("Sort: {}", app.current_bedrock_sort);
        let left_pane_block = Block::default()
            .title(Line::from(vec![
                Span::styled("Bedrock Models ", Style::default().fg(theme.primary_foreground).bold()),
                Span::styled(filter_label, Style::default().fg(theme.secondary_foreground)),
                Span::raw(" | "),
                Span::styled(sort_label, Style::default().fg(theme.secondary_foreground)),
            ]))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border_primary));
        let left_pane_content_area = left_pane_block.inner(chunks[0]);
        f.render_widget(left_pane_block, chunks[0]);
        let filtered_models = app.bedrock_model_view_mode.apply(&app.bedrock_models);
        let model_items: Vec<ListItem> = filtered_models
            .iter()
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
            if let Some(model) = filtered_models.get(selected_idx) {
                let model_name = model.model_name().unwrap_or("N/A");
                let model_id = model.model_id();
                let provider_name = model.provider_name().unwrap_or("N/A");
                let customizations = format!("{:?}", model.customizations_supported());
                let inference_types = format!("{:?}", model.inference_types_supported());
                let streaming = format!("{}", model.response_streaming_supported().unwrap_or(false));
                let details_lines = vec![
                    Line::from(vec![Span::styled("Name: ", theme.ollama_model_list_details_title.clone()), Span::raw(model_name)]),
                    Line::from(vec![Span::styled("ID: ", theme.ollama_model_list_details_title.clone()), Span::raw(model_id)]),
                    Line::from(vec![Span::styled("Provider: ", theme.ollama_model_list_details_title.clone()), Span::raw(provider_name)]),
                    Line::from(vec![Span::styled("Customizations: ", theme.ollama_model_list_details_title.clone()), Span::raw(customizations)]),
                    Line::from(vec![Span::styled("Inference Types: ", theme.ollama_model_list_details_title.clone()), Span::raw(inference_types)]),
                    Line::from(vec![Span::styled("Response Streaming: ", theme.ollama_model_list_details_title.clone()), Span::raw(streaming)]),
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