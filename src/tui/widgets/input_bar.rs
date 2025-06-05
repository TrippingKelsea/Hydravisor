// src/tui/widgets/input_bar.rs
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use crate::tui::{App, InputMode};
use textwrap;

pub struct InputBarWidget;

const CURSOR_CHAR_STR: &str = "|"; // Changed to line cursor
const CURSOR_CHAR: char = '|';   // Changed to line cursor

impl InputBarWidget {
    pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
        let theme = &app.theme;

        let title = if let Some(model_name) = &app.editing_system_prompt_for_model {
            Line::from(vec![
                Span::styled("Editing System Prompt for ", theme.input_bar_title),
                Span::styled(model_name.clone(), theme.input_bar_title.patch(Style::default().add_modifier(Modifier::BOLD))),
                Span::styled(":", theme.input_bar_title), // Removed Ctrl+Up/Down mention
            ])
        } else if app.active_view == crate::tui::TuiView::Chat && app.active_chat.is_some() && app.input_mode == InputMode::Editing {
            Line::from(Span::styled("Chat Input (Esc: Normal Mode):", theme.input_bar_title)) // Removed Ctrl+Up/Down mention
        } else {
            Line::from(Span::styled("Input:", theme.input_bar_title)) 
        };
        
        let input_block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(if app.input_mode == InputMode::Editing || app.editing_system_prompt_for_model.is_some() {
                theme.border_accent 
            } else {
                theme.border_secondary
            }));
        
        let text_area_width = area.width.saturating_sub(2).max(1); 
        app.last_input_text_area_width = text_area_width; // Cache the width
        app.input_bar_visible_height = area.height.saturating_sub(2).max(1); 

        let is_editing_mode = app.input_mode == InputMode::Editing || app.editing_system_prompt_for_model.is_some();
        
        let text_for_wrapping: String;
        let cursor_char_actual_idx_in_text_for_wrapping: usize; // Actual index of CURSOR_CHAR in text_for_wrapping

        if is_editing_mode {
            let mut current_chars: Vec<char> = app.current_input.chars().collect();
            // app.input_cursor_char_idx is the logical position in app.current_input
            // This becomes the actual index of CURSOR_CHAR when inserted.
            cursor_char_actual_idx_in_text_for_wrapping = app.input_cursor_char_idx.min(current_chars.len());
            current_chars.insert(cursor_char_actual_idx_in_text_for_wrapping, CURSOR_CHAR);
            text_for_wrapping = current_chars.into_iter().collect::<String>();
        } else {
            text_for_wrapping = app.current_input.clone();
            cursor_char_actual_idx_in_text_for_wrapping = 0; // Not used if not editing, but provide a default
        }

        let wrapped_lines_as_strings: Vec<String> = textwrap::wrap(&text_for_wrapping, text_area_width as usize)
            .iter()
            .map(|s| s.to_string())
            .collect();
        
        app.input_bar_last_wrapped_line_count = wrapped_lines_as_strings.len();

        if app.input_bar_cursor_needs_to_be_visible && is_editing_mode {
            let mut char_count_so_far = 0;
            let mut calculated_cursor_line = 0; 

            if !wrapped_lines_as_strings.is_empty() {
                for (i, line_string) in wrapped_lines_as_strings.iter().enumerate() {
                    let line_char_len = line_string.chars().count();
                    
                    // Check if the cursor_char_actual_idx_in_text_for_wrapping falls within this line's span
                    // The span is [char_count_so_far, char_count_so_far + line_char_len)
                    if cursor_char_actual_idx_in_text_for_wrapping >= char_count_so_far && 
                       cursor_char_actual_idx_in_text_for_wrapping < char_count_so_far + line_char_len {
                        calculated_cursor_line = i;
                        break; 
                    }
                    // Edge case: if cursor is at the very end of text_for_wrapping, and this is the last line string
                    if i == wrapped_lines_as_strings.len() - 1 && 
                       cursor_char_actual_idx_in_text_for_wrapping == char_count_so_far + line_char_len {
                        calculated_cursor_line = i;
                        break;
                    }
                    char_count_so_far += line_char_len;
                }
            } else { // No wrapped lines (e.g. text_for_wrapping is empty)
                calculated_cursor_line = 0;
            }
            
            // Ensure calculated_cursor_line is valid if lines exist
            if !wrapped_lines_as_strings.is_empty() {
                calculated_cursor_line = calculated_cursor_line.min(wrapped_lines_as_strings.len() -1);
            }

            if app.input_bar_visible_height > 0 {
                let desired_scroll_top = calculated_cursor_line
                    .saturating_sub((app.input_bar_visible_height as usize).saturating_sub(1));
                app.input_bar_scroll = desired_scroll_top as u16;
            } else {
                app.input_bar_scroll = 0;
            }
            app.input_bar_cursor_needs_to_be_visible = false; 
        }

        let max_scroll = (app.input_bar_last_wrapped_line_count
            .saturating_sub(app.input_bar_visible_height as usize)) as u16;
        app.input_bar_scroll = app.input_bar_scroll.min(max_scroll);
        
        let display_lines: Vec<Line> = wrapped_lines_as_strings
            .iter()
            .map(|s| Line::from(s.as_str()))
            .collect();
        let text_to_display = Text::from(display_lines);

        let paragraph = Paragraph::new(text_to_display)
            .block(input_block)
            .style(Style::default().fg(theme.input_bar_text))
            .scroll((app.input_bar_scroll, 0));
        
        f.render_widget(paragraph, area);
    }
}
