use ratatui::{
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::Style,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use crate::tui::App;

pub struct NewVmPopupWidget;

impl NewVmPopupWidget {
    pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
        let theme = &app.theme;

        let popup_area = centered_rect(80, 80, area);
        
        let block = Block::default()
            .title("Create New Virtual Machine")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border_accent))
            .title_alignment(Alignment::Center)
            .style(Style::default().bg(theme.input_bar_background));
        
        f.render_widget(Clear, popup_area); // Clear the area before rendering the popup
        f.render_widget(block.clone(), popup_area);

        let inner_area = block.inner(popup_area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3), // Name
                Constraint::Length(3), // Source Image
                Constraint::Length(3), // Disk Path
                Constraint::Length(3), // CPU
                Constraint::Length(3), // RAM
                Constraint::Length(3), // Disk Size
                Constraint::Length(1), // Spacer
                Constraint::Length(1), // ISO Checkbox
                Constraint::Length(3), // ISO Path
                Constraint::Min(1),    // Spacer
                Constraint::Length(1), // Instructions
            ].as_ref())
            .split(inner_area);
        
        let active_input_style = Style::default().fg(theme.highlight_style.fg.unwrap_or(theme.primary_foreground));

        let mut name_input = Paragraph::new(app.new_vm_name.as_str())
            .block(Block::default().borders(Borders::ALL).title("VM Name"));
        if app.active_new_vm_input_idx == 0 {
            name_input = name_input.style(active_input_style);
        }
        
        let mut source_image_input = Paragraph::new(app.new_vm_source_image_path.as_str())
            .block(Block::default().borders(Borders::ALL).title("Source Image Path (optional)"));
        if app.active_new_vm_input_idx == 1 {
            source_image_input = source_image_input.style(active_input_style);
        }

        let mut disk_path_input = Paragraph::new(app.new_vm_disk_path.as_str())
            .block(Block::default().borders(Borders::ALL).title("Disk Image Path"));
        if app.active_new_vm_input_idx == 2 {
            disk_path_input = disk_path_input.style(active_input_style);
        }
        
        let mut cpu_input = Paragraph::new(app.new_vm_cpu.as_str())
            .block(Block::default().borders(Borders::ALL).title("CPUs"));
        if app.active_new_vm_input_idx == 3 {
            cpu_input = cpu_input.style(active_input_style);
        }
        
        let mut ram_input = Paragraph::new(app.new_vm_ram_mb.as_str())
            .block(Block::default().borders(Borders::ALL).title("Memory (e.g., 4GB or 4096MB)"));
        if app.active_new_vm_input_idx == 4 {
            ram_input = ram_input.style(active_input_style);
        }
        
        let mut disk_size_input = Paragraph::new(app.new_vm_disk_gb.as_str())
            .block(Block::default().borders(Borders::ALL).title("Disk Size (GB)"));
        if app.active_new_vm_input_idx == 5 {
            disk_size_input = disk_size_input.style(active_input_style);
        }

        let iso_checkbox_text = if app.new_vm_use_iso { "[x] Boot from ISO" } else { "[ ] Boot from ISO" };
        let mut iso_checkbox = Paragraph::new(iso_checkbox_text);
        if app.active_new_vm_input_idx == 6 {
            iso_checkbox = iso_checkbox.style(active_input_style);
        }

        let mut iso_path_input = Paragraph::new(app.new_vm_iso_path.as_str())
            .block(Block::default().borders(Borders::ALL).title("ISO Path"));

        if !app.new_vm_use_iso {
            iso_path_input = iso_path_input.style(Style::default().fg(theme.tertiary_foreground));
        }
        if app.active_new_vm_input_idx == 7 && app.new_vm_use_iso {
            iso_path_input = iso_path_input.style(active_input_style);
        }

        f.render_widget(name_input, chunks[0]);
        f.render_widget(source_image_input, chunks[1]);
        f.render_widget(disk_path_input, chunks[2]);
        f.render_widget(cpu_input, chunks[3]);
        f.render_widget(ram_input, chunks[4]);
        f.render_widget(disk_size_input, chunks[5]);
        f.render_widget(iso_checkbox, chunks[7]);
        f.render_widget(iso_path_input, chunks[8]);
        
        let instructions = Paragraph::new("Press Tab to switch fields, Space to toggle checkbox, Enter to create, Esc to cancel.")
            .style(Style::default().fg(theme.secondary_foreground))
            .alignment(Alignment::Center);
        f.render_widget(instructions, chunks[10]);
    }
}

/// Helper for creating a centered popup.
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
} 