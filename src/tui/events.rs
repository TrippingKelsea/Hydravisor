// src/tui/events.rs

use anyhow::Result;
use crossterm::event::{Event as CrosstermEvent, KeyCode, KeyEvent, MouseEvent, MouseEventKind, EventStream};
use futures::StreamExt;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io::Stdout;
use std::time::{Duration, Instant};
use tracing::{error};
use std::sync::Arc;

use super::app::{App, AppEvent, AppView, ChatMessage, ChatSession, InputMode};
use super::ui::ui;

#[cfg(feature = "bedrock_integration")]
use aws_sdk_bedrock::types::FoundationModelSummary;

pub async fn run_app_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    mut app: App,
) -> Result<()> {
    let mut last_tick = Instant::now();
    let mut event_receiver = app.event_receiver.take().unwrap();
    let mut crossterm_events = EventStream::new();

    // Initial data fetch
    app.fetch_vms().await;
    #[cfg(feature = "ollama_integration")]
    app.fetch_ollama_models().await;
    #[cfg(feature = "bedrock_integration")]
    app.fetch_bedrock_models().await;

    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        // --- Log Handling ---
        if let Some(ref mut receiver) = app.log_receiver {
            while let Ok(log_entry) = receiver.try_recv() {
                app.log_entries.push(log_entry);
            }
        }
        let max_logs = 1000;
        if app.log_entries.len() > max_logs {
            let overflow = app.log_entries.len() - max_logs;
            app.log_entries.drain(0..overflow);
        }
        if app.active_view == AppView::Logs {
            let is_scrolled_to_bottom = match app.log_list_state.selected() {
                Some(index) => index >= app.log_entries.len().saturating_sub(1),
                None => true,
            };
            if is_scrolled_to_bottom && !app.log_entries.is_empty() {
                    app.log_list_state.select(Some(app.log_entries.len() - 1));
            }
        }

        let tick_duration = Duration::from_millis(app.config.interface.refresh_interval_ms);

        tokio::select! {
            // Handle app events from the channel
            Some(event) = event_receiver.recv() => {
                match event {
                    AppEvent::FetchVms => {
                        app.fetch_vms().await;
                    }
                    #[cfg(feature = "ollama_integration")]
                    AppEvent::FetchOllamaModels => {
                        app.fetch_ollama_models().await;
                    }
                    #[cfg(feature = "bedrock_integration")]
                    AppEvent::FetchBedrockModels => {
                        app.fetch_bedrock_models().await;
                    }
                    AppEvent::DestroyVm(vm_name) => {
                        let libvirt_manager = Arc::clone(&app.libvirt_manager);
                        tokio::spawn(async move {
                            if let Err(e) = libvirt_manager.lock().await.destroy_vm(&vm_name) {
                                error!("Failed to destroy VM '{}': {}", &vm_name, e);
                            }
                            // Need to trigger a refresh. For now, rely on tick or user action.
                        });
                        app.event_sender.send(AppEvent::FetchVms).unwrap(); // Trigger refresh
                    }
                    AppEvent::ResumeVm(vm_name) => {
                        let libvirt_manager = Arc::clone(&app.libvirt_manager);
                        tokio::spawn(async move {
                            if let Err(e) = libvirt_manager.lock().await.resume_vm(&vm_name) {
                                error!("Failed to start VM '{}': {}", &vm_name, e);
                            }
                        });
                    }
                }
            }

            // Handle terminal events
            Some(Ok(event)) = crossterm_events.next() => {
                match event {
                    CrosstermEvent::Key(key) => on_key(&mut app, key),
                    CrosstermEvent::Mouse(mouse) => on_mouse_event(&mut app, mouse),
                    _ => {}
                }
            }
            
            // Handle tick for periodic updates
            _ = tokio::time::sleep(tick_duration) => {
                 if last_tick.elapsed() >= tick_duration {
                    app.tick();
                    app.event_sender.send(AppEvent::FetchVms).unwrap(); // Send event to refresh
                    #[cfg(feature = "ollama_integration")]
                    app.event_sender.send(AppEvent::FetchOllamaModels).unwrap(); // Send event to refresh
                    #[cfg(feature = "bedrock_integration")]
                    app.event_sender.send(AppEvent::FetchBedrockModels).unwrap();
                    last_tick = Instant::now();
                }
            }
        }
        
        if app.should_quit {
            return Ok(());
        }
    }
}

pub fn on_tick(_app: &mut App) {
    // This is now handled in app.tick()
}

pub fn on_mouse_event(app: &mut App, mouse_event: MouseEvent) {
    match mouse_event.kind {
        MouseEventKind::ScrollUp => {
            match app.active_view {
                AppView::VmList => app.select_previous_item_in_vm_list(),
                AppView::OllamaModelList => app.select_previous_item_in_ollama_list(),
                #[cfg(feature = "bedrock_integration")]
                AppView::BedrockModelList => app.select_previous_item_in_bedrock_list(),
                AppView::Chat => app.scroll_chat_up(),
                AppView::Logs => app.scroll_logs_up(),
            }
        }
        MouseEventKind::ScrollDown => {
            match app.active_view {
                AppView::VmList => app.select_next_item_in_vm_list(),
                AppView::OllamaModelList => app.select_next_item_in_ollama_list(),
                #[cfg(feature = "bedrock_integration")]
                AppView::BedrockModelList => app.select_next_item_in_bedrock_list(),
                AppView::Chat => app.scroll_chat_down(),
                AppView::Logs => app.scroll_logs_down(),
            }
        }
        _ => {}
    }
}

fn key_matches(app: &App, action: &str, key_event: &KeyEvent) -> bool {
    if let Some((code, mods)) = app.keybinding_map.get(action) {
        key_event.code == *code && key_event.modifiers == *mods
    } else {
        false
    }
}

pub fn on_key(app: &mut App, key_event: KeyEvent) {
    if app.show_keybindings_modal {
        if key_matches(app, "help", &key_event) || key_event.code == KeyCode::Esc {
            app.show_keybindings_modal = false;
        }
        return;
    }
    if app.show_about_modal {
        if key_matches(app, "help", &key_event) || key_event.code == KeyCode::Esc || key_event.code == KeyCode::Char('q') {
            app.show_about_modal = false;
        }
        return;
    }
    if app.show_menu {
        match app.menu_level {
            0 => { // Main Menu
                if key_matches(app, "menu", &key_event) || key_event.code == KeyCode::Esc {
                    app.show_menu = false;
                } else if key_matches(app, "down", &key_event) || key_event.code == KeyCode::Char('j') {
                    app.menu_next();
                } else if key_matches(app, "up", &key_event) || key_event.code == KeyCode::Char('k') {
                    app.menu_previous();
                } else if key_matches(app, "enter", &key_event) || key_event.code == KeyCode::Enter {
                    if let Some(selected) = app.menu_state.selected() {
                        let item_name = match selected {
                            0 => "About",
                            1 => "Preferences",
                            2 => "Quit",
                            _ => "",
                        };
                        match item_name {
                            "About" => {
                                app.show_about_modal = true;
                                app.show_menu = false;
                            },
                            "Preferences" => {
                                app.menu_level = 1;
                                app.menu_sub_state.select(Some(0));
                            },
                            "Quit" => app.should_quit = true,
                            _ => {}
                        }
                    }
                }
            },
            1 => { // Preferences Submenu
                if key_matches(app, "menu", &key_event) || key_event.code == KeyCode::Esc {
                    app.menu_level = 0;
                    app.menu_state.select(Some(1)); // Reselect "Preferences"
                } else if key_matches(app, "down", &key_event) || key_event.code == KeyCode::Char('j') {
                    app.menu_next();
                } else if key_matches(app, "up", &key_event) || key_event.code == KeyCode::Char('k') {
                    app.menu_previous();
                } else if key_matches(app, "enter", &key_event) || key_event.code == KeyCode::Enter {
                     if let Some(selected) = app.menu_sub_state.selected() {
                        let item_name = match selected {
                            0 => "Key Bindings",
                            1 => "Back",
                            _ => "",
                        };
                        match item_name {
                            "Key Bindings" => {
                                app.show_keybindings_modal = true;
                                app.show_menu = false;
                            },
                            "Back" => {
                                app.menu_level = 0;
                                app.menu_state.select(Some(1));
                            }
                            _ => {}
                        }
                    }
                }
            },
            _ => {} // Should not happen
        }
        return;
    }

    match app.input_mode {
        InputMode::Normal => handle_normal_mode_key(app, key_event),
        InputMode::Editing => handle_editing_mode_key(app, key_event),
        InputMode::VmWizard => handle_vm_wizard_mode_key(app, key_event),
        InputMode::ConfirmingDestroy => handle_confirm_destroy_mode_key(app, key_event),
    }
}

fn handle_normal_mode_key(app: &mut App, key_event: KeyEvent) {
    if key_matches(app, "quit", &key_event) {
        app.should_quit = true;
    } else if key_matches(app, "next_tab", &key_event) {
        app.active_view = app.active_view.next();
    } else if key_matches(app, "prev_tab", &key_event) {
        app.active_view = app.active_view.previous();
    } else if key_matches(app, "menu", &key_event) {
        app.show_menu = !app.show_menu;
        if app.show_menu {
            app.menu_state.select(Some(0));
            app.menu_level = 0;
        }
    } else if key_matches(app, "down", &key_event) || key_event.code == KeyCode::Char('j') {
        match app.active_view {
            AppView::VmList => app.select_next_item_in_vm_list(),
            AppView::OllamaModelList => app.select_next_item_in_ollama_list(),
            #[cfg(feature = "bedrock_integration")]
            AppView::BedrockModelList => app.select_next_item_in_bedrock_list(),
            AppView::Chat => app.scroll_chat_down(),
            AppView::Logs => app.scroll_logs_down(),
        }
    } else if key_matches(app, "up", &key_event) || key_event.code == KeyCode::Char('k') {
        match app.active_view {
            AppView::VmList => app.select_previous_item_in_vm_list(),
            AppView::OllamaModelList => app.select_previous_item_in_ollama_list(),
            #[cfg(feature = "bedrock_integration")]
            AppView::BedrockModelList => app.select_previous_item_in_bedrock_list(),
            AppView::Chat => app.scroll_chat_up(),
            AppView::Logs => app.scroll_logs_up(),
        }
    } else if key_matches(app, "enter", &key_event) {
        match app.active_view {
            #[cfg(feature = "ollama_integration")]
            AppView::OllamaModelList => {
                if let Some(selected_index) = app.ollama_model_list_state.selected() {
                    let selected_model = &app.ollama_models[selected_index];
                    let selected_model_name = selected_model.name.clone();
                    if app.active_chat.as_ref().map_or(true, |c| c.model_name != selected_model_name) {
                        app.active_chat = Some(ChatSession {
                            model_name: selected_model_name.clone(),
                            messages: vec![ChatMessage {
                                sender: "System".to_string(),
                                content: app.get_active_system_prompt(&selected_model_name),
                                timestamp: "".to_string(),
                                thought: None,
                            }],
                            is_streaming: false,
                        });
                    }
                    app.active_view = AppView::Chat;
                    app.chat_list_state.select(None);
                }
            },
            _ => {}
        }
    } else if key_matches(app, "edit", &key_event) {
        match app.active_view {
            #[cfg(feature = "ollama_integration")]
            AppView::OllamaModelList => {
                if let Some(selected_index) = app.ollama_model_list_state.selected() {
                    let model_name = app.ollama_models[selected_index].name.clone();
                    let prompt = app.get_active_system_prompt(&model_name);
                    app.editing_system_prompt_for_model = Some(model_name);
                    app.current_input = prompt;
                    app.input_mode = InputMode::Editing;
                    app.reset_cursor_position();
                }
            },
            _ => {}
        }
    } else if key_matches(app, "refresh", &key_event) {
        app.event_sender.send(AppEvent::FetchVms).unwrap();
        #[cfg(feature = "ollama_integration")]
        app.event_sender.send(AppEvent::FetchOllamaModels).unwrap();
        #[cfg(feature = "bedrock_integration")]
        app.event_sender.send(AppEvent::FetchBedrockModels).unwrap();
    } else if key_matches(app, "delete", &key_event) {
        match app.active_view {
            AppView::VmList => {
                if let Some(selected_index) = app.vm_list_state.selected() {
                    let vm_name = app.vms[selected_index].name.clone();
                    app.vm_to_destroy = Some(vm_name);
                    app.input_mode = InputMode::ConfirmingDestroy;
                }
            }
            _ => {}
        }
    } else if key_matches(app, "new_vm", &key_event) {
        app.show_new_vm_popup = true;
        app.input_mode = InputMode::VmWizard;
        app.active_new_vm_input_idx = 0;
    }

    // View-specific key handling for Bedrock
    #[cfg(feature = "bedrock_integration")]
    if app.active_view == AppView::BedrockModelList {
        if key_matches(app, "bedrock_filter", &key_event) {
            // Manually define the available filters since they are struct fields, not a map
            let filters = ["available_to_use", "available_to_request_access"];
            let idx = filters.iter().position(|&f| f == app.current_bedrock_filter).unwrap_or(0);
            let next_idx = (idx + 1) % filters.len();
            app.current_bedrock_filter = filters[next_idx].to_string();
        } else if key_matches(app, "bedrock_sort", &key_event) {
            // Currently, only one sort is implemented, so we can just log or do nothing.
            // When more are added, this can cycle like the filters.
            let sorts = ["alphabetical"]; // The only sort option for now
            let idx = sorts.iter().position(|&s| s == app.current_bedrock_sort).unwrap_or(0);
            let next_idx = (idx + 1) % sorts.len();
            app.current_bedrock_sort = sorts[next_idx].to_string();
        }
    }
}

fn handle_editing_mode_key(app: &mut App, key_event: KeyEvent) {
    match key_event.code {
        KeyCode::Enter => {
            if let Some(model_name) = app.editing_system_prompt_for_model.take() {
                app.editable_ollama_model_prompts.insert(model_name, app.current_input.clone());
            }
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Char(c) => {
            let char_idx = app.input_cursor_char_idx;
            app.current_input.insert(char_idx, c);
            app.input_cursor_char_idx += 1;
        }
        KeyCode::Backspace => {
            if app.input_cursor_char_idx > 0 {
                app.input_cursor_char_idx -= 1;
                let char_idx = app.input_cursor_char_idx;
                app.current_input.remove(char_idx);
            }
        }
        KeyCode::Left => {
            if app.input_cursor_char_idx > 0 {
                app.input_cursor_char_idx -= 1;
            }
        }
        KeyCode::Right => {
            if app.input_cursor_char_idx < app.current_input.len() {
                app.input_cursor_char_idx += 1;
            }
        }
        KeyCode::Esc => {
            app.input_mode = InputMode::Normal;
            app.editing_system_prompt_for_model = None;
        }
        // Add Up/Down arrow handling later if needed for multi-line
        _ => {}
    }
     app.input_bar_cursor_needs_to_be_visible = true;
}


fn handle_vm_wizard_mode_key(app: &mut App, key_event: KeyEvent) {
    let current_field = match app.active_new_vm_input_idx {
        0 => &mut app.new_vm_name,
        1 => {
            if key_event.code == KeyCode::Enter || key_event.code == KeyCode::Char(' ') {
                app.new_vm_use_iso = !app.new_vm_use_iso;
            }
            // Dummy mutable ref for the match arm
            &mut String::new()
        },
        2 if app.new_vm_use_iso => &mut app.new_vm_iso_path,
        2 if !app.new_vm_use_iso => &mut app.new_vm_source_image_path,
        3 => &mut app.new_vm_disk_path,
        4 => &mut app.new_vm_cpu,
        5 => &mut app.new_vm_ram_mb,
        6 => &mut app.new_vm_disk_gb,
        _ => return,
    };

    match key_event.code {
        KeyCode::Char(c) => {
            // Only modify string fields
            if app.active_new_vm_input_idx != 1 {
                 current_field.push(c);
            }
        },
        KeyCode::Backspace => {
            if app.active_new_vm_input_idx != 1 {
                current_field.pop();
            }
        },
        KeyCode::Tab => {
            app.active_new_vm_input_idx = (app.active_new_vm_input_idx + 1) % 7;
        },
        KeyCode::BackTab => {
            app.active_new_vm_input_idx = (app.active_new_vm_input_idx + 6) % 7;
        },
        KeyCode::Enter => {
            // Check if on the checkbox, if so, Tab acts as Enter for other fields
             if app.active_new_vm_input_idx != 1 {
                app.active_new_vm_input_idx = (app.active_new_vm_input_idx + 1) % 7;
             }
        }
        KeyCode::Esc => {
            app.show_new_vm_popup = false;
            app.input_mode = InputMode::Normal;
        },
        _ => {}
    }
}


fn handle_confirm_destroy_mode_key(app: &mut App, key_event: KeyEvent) {
    match key_event.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            if let Some(vm_name) = app.vm_to_destroy.take() {
                app.event_sender.send(AppEvent::DestroyVm(vm_name)).unwrap();
            }
            app.input_mode = InputMode::Normal;
        },
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.vm_to_destroy = None;
            app.input_mode = InputMode::Normal;
        }
        _ => {}
    }
} 