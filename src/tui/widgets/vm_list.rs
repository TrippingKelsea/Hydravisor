use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use crate::tui::App;

pub struct VmListWidget;

impl VmListWidget {
    pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
        // Create layout for left (VM list) and right (VM details) panes
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(40), // Left pane for VM list
                Constraint::Percentage(60), // Right pane for VM details
            ].as_ref())
            .split(area);

        // Left Pane: VM List
        let left_pane_block = Block::default().title("VMs").borders(Borders::ALL);
        let left_pane_content_area = left_pane_block.inner(chunks[0]);
        f.render_widget(left_pane_block, chunks[0]);

        let vm_items: Vec<ListItem> = app.vms.iter()
            .map(|vm| ListItem::new(format!("{} ({}) - {:?}", vm.name, vm.instance_id, vm.state)))
            .collect();
        let vm_list = List::new(vm_items)
            .highlight_style(Style::default().add_modifier(Modifier::BOLD).bg(Color::Gray))
            .highlight_symbol(">> ");
        f.render_stateful_widget(vm_list, left_pane_content_area, &mut app.vm_list_state);

        // Right Pane: VM Details
        let right_pane_block = Block::default().title("VM Details").borders(Borders::ALL);
        let right_pane_content_area = right_pane_block.inner(chunks[1]);
        f.render_widget(right_pane_block, chunks[1]);

        if let Some(selected_idx) = app.vm_list_state.selected() {
            if let Some(vm) = app.vms.get(selected_idx) {
                let details = format!("Name: {}\nID: {}\nState: {:?}\nType: {:?}\nCPUs: {:?}\nMax Mem: {:?} KB\nUsed Mem: {:?} KB",
                    vm.name, vm.instance_id, vm.state, vm.env_type, vm.cpu_cores_used, vm.memory_max_kb, vm.memory_used_kb);
                f.render_widget(Paragraph::new(Text::from(details)), right_pane_content_area);
            }
        } else {
            f.render_widget(Paragraph::new("No VM selected"), right_pane_content_area);
        }
    }
}
