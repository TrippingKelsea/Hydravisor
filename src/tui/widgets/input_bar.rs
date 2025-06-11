// src/tui/widgets/input_bar.rs
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use crate::tui::app::{App, InputMode, AppView};
use textwrap;

pub struct InputBarWidget;

const CURSOR_CHAR: char = '|';

impl InputBarWidget {
    pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
        let theme = &app.theme;

        let is_editing_mode = app.input_mode == InputMode::Editing || app.editing_system_prompt_for_model.is_some();
        
        let title = if let Some(model_name) = &app.editing_system_prompt_for_model {
            Line::from(vec![
                Span::styled("Editing System Prompt for ", theme.input_bar_title),
                Span::styled(model_name.clone(), theme.input_bar_title.patch(Style::default().add_modifier(Modifier::BOLD))),
                Span::styled(":", theme.input_bar_title),
            ])
        } else if app.active_view == AppView::Chat && app.active_chat.is_some() && is_editing_mode {
            Line::from(Span::styled("Chat Input (Esc: Normal Mode):", theme.input_bar_title))
        } else {
            Line::from(Span::styled("Input:", theme.input_bar_title)) 
        };
        
        let input_block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(if is_editing_mode {
                theme.border_accent 
            } else {
                theme.border_secondary
            }));
        
        let text_area_width = area.width.saturating_sub(2).max(1); 
        app.last_input_text_area_width = text_area_width;
        app.input_bar_visible_height = area.height.saturating_sub(2).max(1); 

        // 1. Create the text to be displayed and wrapped, with the cursor character inserted.
        let text_for_wrapping = if is_editing_mode {
            let mut current_chars: Vec<char> = app.current_input.chars().collect();
            let cursor_pos = app.input_cursor_char_idx.min(current_chars.len());
            current_chars.insert(cursor_pos, CURSOR_CHAR);
            current_chars.into_iter().collect::<String>()
        } else {
            app.current_input.clone()
        };
        
        // 2. Wrap the text. This is the single source of truth for all calculations.
        let wrapped_lines: Vec<String> = textwrap::wrap(&text_for_wrapping, text_area_width as usize)
            .iter()
            .map(|s| s.to_string())
            .collect();
        app.input_bar_last_wrapped_line_count = wrapped_lines.len();

        // 3. Find the cursor's line and adjust scroll if needed.
        if app.input_bar_cursor_needs_to_be_visible && is_editing_mode {
            // Find the line containing the cursor character.
            let mut calculated_cursor_line = wrapped_lines
                .iter()
                .position(|line| line.contains(CURSOR_CHAR))
                .unwrap_or(0); // Default to line 0 if not found, though it always should be.
            
            // Ensure the calculated line is valid, especially for an empty input which might result in an empty `wrapped_lines`.
            if !wrapped_lines.is_empty() {
                calculated_cursor_line = calculated_cursor_line.min(wrapped_lines.len().saturating_sub(1));
            }

            // "Scroll into view" logic.
            if app.input_bar_visible_height > 0 {
                let current_scroll_top = app.input_bar_scroll as usize;
                let current_scroll_bottom = current_scroll_top + (app.input_bar_visible_height as usize).saturating_sub(1);

                if calculated_cursor_line < current_scroll_top {
                    app.input_bar_scroll = calculated_cursor_line as u16;
                } else if calculated_cursor_line > current_scroll_bottom {
                    app.input_bar_scroll = (calculated_cursor_line.saturating_sub(app.input_bar_visible_height.saturating_sub(1) as usize)) as u16;
                }
            } else {
                app.input_bar_scroll = 0;
            }
            app.input_bar_cursor_needs_to_be_visible = false; 
        }

        // 4. Clamp scroll to the maximum possible value based on the final wrapped lines.
        let max_scroll = (app.input_bar_last_wrapped_line_count as u16)
            .saturating_sub(app.input_bar_visible_height);
        app.input_bar_scroll = app.input_bar_scroll.min(max_scroll);
        
        // 5. Render the paragraph.
        let display_lines: Vec<Line> = wrapped_lines
            .iter()
            .map(|s| Line::from(s.as_str()))
            .collect();
        let text_to_display = Text::from(display_lines);

        let paragraph = Paragraph::new(text_to_display)
            .block(input_block)
            .style(Style::default().fg(theme.primary_foreground))
            .scroll((app.input_bar_scroll, 0));
        
        f.render_widget(paragraph, area);
    }

    pub fn calculate_height(app: &App, width: u16) -> u16 {
        let text_area_width = width.saturating_sub(2).max(1);
        let text = if app.editing_system_prompt_for_model.is_some() || app.input_mode == InputMode::Editing {
            &app.current_input
        } else {
            // In normal mode, if there's no active editing, we can consider it empty
            // for layout purposes, or show a placeholder. Let's use a single line.
            ""
        };

        if text.is_empty() {
            return 3; // Default height for an empty input bar
        }

        let wrapped_lines = textwrap::wrap(text, text_area_width as usize).len();
        (wrapped_lines as u16).max(1).saturating_add(2).min(10) // Add 2 for borders, max height 10
    }
}
