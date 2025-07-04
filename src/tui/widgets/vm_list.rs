use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};
use crate::tui::App;
use crate::libvirt_manager::VmState;

pub struct VmListWidget;

impl VmListWidget {
    pub fn render(f: &mut Frame, app: &mut App, area: Rect) {
        let theme = &app.theme;

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(40), // Left pane for VM list
                Constraint::Percentage(60), // Right pane for VM details
            ].as_ref())
            .split(area);

        // Left Pane: VM List
        let left_pane_block = Block::default()
            .title(Line::from(Span::styled("VMs", Style::default().fg(theme.primary_foreground).bold())))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border_primary));
        let left_pane_content_area = left_pane_block.inner(chunks[0]);
        f.render_widget(left_pane_block, chunks[0]);

        let vm_items: Vec<ListItem> = app.vms.iter()
            .map(|vm| {
                let state_style = match vm.state {
                    VmState::Running => theme.vm_list_status_running,
                    VmState::Stopped => theme.vm_list_status_stopped,
                    VmState::Suspended => theme.vm_list_status_other,
                    _ => theme.vm_list_status_other,
                };
                let content = Line::from(vec![
                    Span::styled(format!("{} ", vm.name), Style::default().fg(theme.primary_foreground)),
                    Span::styled(format!("({:.7})", vm.instance_id), Style::default().fg(theme.secondary_foreground)),
                    Span::raw(" - "),
                    Span::styled(format!("{:?}", vm.state), state_style),
                ]);
                ListItem::new(content)
            })
            .collect();
        
        let vm_list = List::new(vm_items)
            .highlight_style(Style::default().fg(theme.list_highlight_fg).bg(theme.list_highlight_bg))
            .highlight_symbol(">> ");
        f.render_stateful_widget(vm_list, left_pane_content_area, &mut app.vm_list_state);

        // Right Pane: VM Details
        let right_pane_block = Block::default()
            .title(Line::from(Span::styled("VM Details", Style::default().fg(theme.primary_foreground))))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border_secondary));
        let right_pane_content_area = right_pane_block.inner(chunks[1]);
        f.render_widget(right_pane_block, chunks[1]);

        if let Some(selected_idx) = app.vm_list_state.selected() {
            if let Some(vm) = app.vms.get(selected_idx) {
                let details_text = vec![
                    Line::from(vec![Span::styled("Name: ", Style::default().fg(theme.secondary_foreground)), Span::raw(&vm.name)]),
                    Line::from(vec![Span::styled("ID:   ", Style::default().fg(theme.secondary_foreground)), Span::raw(format!("{}", vm.instance_id))]),
                    Line::from(vec![Span::styled("State: ", Style::default().fg(theme.secondary_foreground)), Span::styled(format!("{:?}", vm.state), match vm.state {
                        VmState::Running => theme.vm_list_status_running,
                        VmState::Stopped => theme.vm_list_status_stopped,
                        VmState::Suspended => theme.vm_list_status_other,
                        _ => theme.vm_list_status_other,
                    })]),
                    Line::from(vec![Span::styled("CPUs: ", Style::default().fg(theme.secondary_foreground)), Span::raw(format!("{:?}", vm.cpu_cores_used.map(|c| c.to_string()).unwrap_or_else(|| "N/A".to_string())))]),
                    Line::from(vec![Span::styled("Max Mem: ", Style::default().fg(theme.secondary_foreground)), Span::raw(format!("{:?} KB", vm.memory_max_kb.map(|m| m.to_string()).unwrap_or_else(|| "N/A".to_string())))]),
                    Line::from(vec![Span::styled("Used Mem: ", Style::default().fg(theme.secondary_foreground)), Span::raw(format!("{:?} KB", vm.memory_used_kb.map(|m| m.to_string()).unwrap_or_else(|| "N/A".to_string())))]),
                ];
                f.render_widget(Paragraph::new(Text::from(details_text)).style(Style::default().fg(theme.primary_foreground)), right_pane_content_area);
            }
        } else {
            f.render_widget(Paragraph::new("No VM selected").style(Style::default().fg(theme.secondary_foreground)), right_pane_content_area);
        }
    }
}
