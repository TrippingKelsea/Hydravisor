use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};

use super::app::App;
use super::widgets::{
    about_modal::AboutModalWidget,
    chat::ChatWidget,
    input_bar::InputBarWidget,
    logs::LogsWidget,
    menu::MenuWidget,
    new_vm_popup::NewVmPopupWidget,
    ollama_model_list::OllamaModelListWidget,
    status_bar::StatusBarWidget,
    vm_list::VmListWidget,
};
#[cfg(feature = "bedrock_integration")]
use super::widgets::bedrock_model_list::BedrockModelListWidget;
use super::app::AppView;

pub fn ui(f: &mut Frame, app: &mut App) {
    // The main layout defines a status bar at the top, content in the middle,
    // and an input bar at the bottom.
    let input_bar_height = InputBarWidget::calculate_height(app, f.size().width);
    let main_layout_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Status bar
            Constraint::Min(0),    // Main content
            Constraint::Length(input_bar_height), // Input bar, dynamically sized
        ])
        .split(f.size());

    StatusBarWidget::render(f, app, main_layout_chunks[0]);
    
    let main_content_area = main_layout_chunks[1];

    // Render the main content based on the active view
    match app.active_view {
        AppView::VmList => {
            VmListWidget::render(f, app, main_content_area);
        }
        AppView::OllamaModelList => {
            OllamaModelListWidget::render(f, app, main_content_area);
        }
        #[cfg(feature = "bedrock_integration")]
        AppView::BedrockModelList => {
            BedrockModelListWidget::render(f, app, main_content_area);
        }
        AppView::Chat => {
            ChatWidget::render(f, app, main_content_area);
        }
        AppView::Logs => {
            LogsWidget::render(f, app, main_content_area);
        }
    }

    InputBarWidget::render(f, app, main_layout_chunks[2]);

    // Render Popups over the main content
    if app.show_new_vm_popup {
        NewVmPopupWidget::render(f, app, f.size());
    }
    if app.show_about_modal {
        AboutModalWidget::render(f, app, f.size());
    }
    if app.show_keybindings_modal {
        use super::widgets::keybindings_modal::KeybindingsModalWidget;
        KeybindingsModalWidget::render(f, app, f.size());
    }

    // Render menu over everything if active
    if app.show_menu {
        MenuWidget::render(f, app, f.size());
    }
} 