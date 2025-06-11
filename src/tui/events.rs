// src/tui/events.rs

use anyhow::Result;
use crossterm::event::{Event as CrosstermEvent, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind, EventStream};
use futures::StreamExt;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io::Stdout;
use std::time::{Duration, Instant};
use tracing::{error};
use chrono::Local;
use std::sync::Arc;

use crate::env_manager::{EnvironmentConfig, EnvironmentType};
use super::app::{App, AppEvent, ChatMessage, ChatSession, ChatStreamEvent, InputMode};
use super::ui::ui;
use super::app::parse_ram_str;

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
        if app.active_view == crate::tui::app::AppView::Logs {
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
                    AppEvent::DestroyVm(vm_name) => {
                        let env_manager = Arc::clone(&app.env_manager);
                        tokio::spawn(async move {
                            if let Err(e) = env_manager.lock().await.destroy_environment(&vm_name) {
                                error!("Failed to destroy VM '{}': {}", &vm_name, e);
                            }
                            // Need to trigger a refresh. For now, rely on tick or user action.
                        });
                        app.event_sender.send(AppEvent::FetchVms).unwrap(); // Trigger refresh
                    }
                    AppEvent::ResumeVm(vm_name) => {
                        let env_manager = Arc::clone(&app.env_manager);
                        tokio::spawn(async move {
                            if let Err(e) = env_manager.lock().await.resume_environment(&vm_name) {
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
                    on_tick(&mut app);
                    app.event_sender.send(AppEvent::FetchVms).unwrap(); // Send event to refresh
                    #[cfg(feature = "ollama_integration")]
                    app.event_sender.send(AppEvent::FetchOllamaModels).unwrap(); // Send event to refresh
                    last_tick = Instant::now();
                }
            }
        }
        
        if app.should_quit {
            return Ok(());
        }
    }
}

pub fn on_tick(app: &mut App) {
    // --- CHAT STREAM HANDLING ---
    if let Some(ref mut receiver) = app.chat_stream_receiver {
        while let Ok(event) = receiver.try_recv() {
            if let Some(session) = app.active_chat.as_mut() {
                match event {
                    ChatStreamEvent::Chunk(chunk) => {
                        if let Some(last_message) = session.messages.last_mut() {
                            if last_message.sender == session.model_name {
                                last_message.content.push_str(&chunk);
                            } else {
                                session.messages.push(ChatMessage {
                                    sender: session.model_name.clone(),
                                    content: chunk,
                                    timestamp: Local::now().format("%H:%M:%S").to_string(),
                                    thought: None,
                                });
                            }
                            } else {
                            session.messages.push(ChatMessage {
                                sender: session.model_name.clone(),
                                content: chunk,
                                timestamp: Local::now().format("%H:%M:%S").to_string(),
                                thought: None,
                            });
                        }
                    },
                    ChatStreamEvent::Error(err_msg) => {
                         session.messages.push(ChatMessage {
                            sender: "System".to_string(),
                            content: format!("Error: {}", err_msg),
                            timestamp: Local::now().format("%H:%M:%S").to_string(),
                            thought: None,
                        });
                        session.is_streaming = false;
                    },
                    ChatStreamEvent::Completed => {
                        session.is_streaming = false;
                    }
                }
            }
        }
    }

    if app.active_view == crate::tui::app::AppView::Chat && app.active_chat.as_ref().map_or(false, |s| s.is_streaming) {
         if !app.active_chat.as_ref().unwrap().messages.is_empty() {
            app.chat_list_state.select(Some(app.active_chat.as_ref().unwrap().messages.len() - 1));
        }
    }
}

pub fn on_mouse_event(app: &mut App, mouse_event: MouseEvent) {
    match mouse_event.kind {
        MouseEventKind::ScrollUp => {
            match app.active_view {
                crate::tui::app::AppView::VmList => app.select_previous_item_in_vm_list(),
                crate::tui::app::AppView::OllamaModelList => app.select_previous_item_in_ollama_list(),
                crate::tui::app::AppView::Chat => app.scroll_chat_up(),
                crate::tui::app::AppView::Logs => app.scroll_logs_up(),
            }
        }
        MouseEventKind::ScrollDown => {
            match app.active_view {
                crate::tui::app::AppView::VmList => app.select_next_item_in_vm_list(),
                crate::tui::app::AppView::OllamaModelList => app.select_next_item_in_ollama_list(),
                crate::tui::app::AppView::Chat => app.scroll_chat_down(),
                crate::tui::app::AppView::Logs => app.scroll_logs_down(),
            }
        }
        _ => {}
    }
}

pub fn on_key(app: &mut App, key_event: KeyEvent) {
    if app.show_about_modal {
        match key_event.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                app.show_about_modal = false;
            }
            _ => {}
        }
        return;
    }

    if app.show_menu {
        match key_event.code {
            KeyCode::Char('h') | KeyCode::Esc => app.show_menu = false,
            KeyCode::Down | KeyCode::Char('j') => app.menu_next(),
            KeyCode::Up | KeyCode::Char('k') => app.menu_previous(),
            KeyCode::Enter => {
                if let Some(selected) = app.menu_state.selected() {
                    match selected {
                        0 => { // About
                            app.show_about_modal = true;
                            app.show_menu = false;
                        },
                        1 => app.should_quit = true, // Quit
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        return;
    }

    if let KeyCode::Char('h') = key_event.code {
        if key_event.modifiers == KeyModifiers::CONTROL {
            app.show_menu = !app.show_menu;
            app.menu_state.select(Some(0));
        return;
    }
    }

    match app.input_mode {
        InputMode::Normal => handle_normal_mode_key(app, key_event),
        InputMode::Editing => handle_editing_mode_key(app, key_event),
        InputMode::VmWizard => handle_vm_wizard_mode_key(app, key_event),
        InputMode::ConfirmingDestroy => handle_confirm_destroy_mode_key(app, key_event),
    }
}

fn handle_normal_mode_key(app: &mut App, key_event: KeyEvent) {
    // Global tab navigation
    if key_event.code == KeyCode::Tab {
        app.active_view = app.active_view.next();
        return;
    }
    if key_event.code == KeyCode::BackTab {
        app.active_view = app.active_view.previous();
        return;
    }

    match app.active_view {
        crate::tui::app::AppView::VmList => match key_event.code {
            KeyCode::Char('q') => app.should_quit = true,
            KeyCode::Char('i') => app.input_mode = InputMode::Editing,
            KeyCode::Char('n') => {
                app.show_new_vm_popup = true;
                app.input_mode = InputMode::VmWizard;
                app.active_new_vm_input_idx = 0;
            },
            KeyCode::Down | KeyCode::Char('j') => app.select_next_item_in_vm_list(),
            KeyCode::Up | KeyCode::Char('k') => app.select_previous_item_in_vm_list(),
            KeyCode::Char('d') => {
                if let Some(selected_index) = app.vm_list_state.selected() {
                    if let Some(vm) = app.vms.get(selected_index) {
                        app.vm_to_destroy = Some(vm.name.clone());
                        app.input_mode = InputMode::ConfirmingDestroy;
                    }
                }
            },
            KeyCode::Enter => {
                if let Some(selected_index) = app.vm_list_state.selected() {
                    if let Some(vm) = app.vms.get(selected_index) {
                        app.event_sender.send(AppEvent::ResumeVm(vm.name.clone())).unwrap();
                    }
                }
            }
            _ => {} 
        },
        crate::tui::app::AppView::OllamaModelList => match key_event.code {
            KeyCode::Char('q') => app.should_quit = true,
            KeyCode::Down | KeyCode::Char('j') => app.select_next_item_in_ollama_list(),
            KeyCode::Up | KeyCode::Char('k') => app.select_previous_item_in_ollama_list(),
            KeyCode::Enter => {
                if let Some(selected_index) = app.ollama_model_list_state.selected() {
                    if let Some(selected_model) = app.ollama_models.get(selected_index) {
                        let selected_model_name = selected_model.name.clone();
                        if app.active_chat.as_ref().map_or(true, |c| c.model_name != selected_model_name) {
                            app.active_chat = Some(ChatSession {
                                model_name: selected_model_name.clone(),
                                messages: Vec::new(),
                                is_streaming: false,
                            });
                            app.active_view = crate::tui::app::AppView::Chat;
                            app.input_mode = InputMode::Editing;
                        }
                    }
                }
            },
            KeyCode::Char('e') => {
                if let Some(selected_index) = app.ollama_model_list_state.selected() {
                    if let Some(selected_model) = app.ollama_models.get(selected_index) {
                        app.editing_system_prompt_for_model = Some(selected_model.name.clone());
                        app.current_input = app.get_active_system_prompt(&selected_model.name);
                        app.input_mode = InputMode::Editing;
                        app.reset_cursor_position();
                    }
                }
            },
            _ => {}
        },
        crate::tui::app::AppView::Chat => match key_event.code {
            KeyCode::Char('q') => app.should_quit = true,
            KeyCode::Char('i') => {
                app.input_mode = InputMode::Editing;
                app.current_input.clear();
                app.reset_cursor_position();
            },
            KeyCode::Down | KeyCode::Char('j') => app.scroll_chat_down(),
            KeyCode::Up | KeyCode::Char('k') => app.scroll_chat_up(),
            _ => {}
        },
        crate::tui::app::AppView::Logs => match key_event.code {
            KeyCode::Char('q') => app.should_quit = true,
            KeyCode::Down | KeyCode::Char('j') => app.scroll_logs_down(),
            KeyCode::Up | KeyCode::Char('k') => app.scroll_logs_up(),
            _ => {}
        },
    }
}

fn handle_editing_mode_key(app: &mut App, key_event: KeyEvent) {
    match key_event.code {
        KeyCode::Esc => {
            if app.editing_system_prompt_for_model.is_some() {
                app.current_input.clear();
                app.editing_system_prompt_for_model = None;
                app.active_view = crate::tui::app::AppView::OllamaModelList;
            }
            app.input_mode = InputMode::Normal;
            app.reset_cursor_position();
        },
        KeyCode::Enter => {
            let input_str = app.current_input.trim().to_string();
            
            if let Some(model_name) = app.editing_system_prompt_for_model.take() {
                app.editable_ollama_model_prompts.insert(model_name, input_str);
                app.current_input.clear();
                app.input_mode = InputMode::Normal;
                app.active_view = crate::tui::app::AppView::OllamaModelList;
                app.reset_cursor_position();
                return;
            }

            if !input_str.is_empty() {
                if let Some(chat_session) = &mut app.active_chat {
                    let user_message = ChatMessage {
                        sender: "User".to_string(),
                        content: input_str.clone(),
                        timestamp: Local::now().format("%H:%M:%S").to_string(),
                        thought: None,
                    };
                    chat_session.messages.push(user_message);
                    chat_session.is_streaming = true;

                    let model_name = chat_session.model_name.clone();
                    let system_prompt = app.get_active_system_prompt(&model_name);

                    let ollama_manager = Arc::clone(&app.ollama_manager);
                    let input_str = app.current_input.trim().to_string();
                    let chat_tx = app.chat_stream_sender.clone();
                    tokio::spawn(async move {
                        let user_message = ChatMessage {
                            sender: "user".to_string(),
                            content: input_str,
                            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
                            thought: None,
                        };
                        let history = vec![user_message];
                        
                        let ollama_manager_guard = ollama_manager.lock().await;
                        match ollama_manager_guard.generate_response_stream(
                            model_name,
                            history,
                            Some(system_prompt),
                        ).await {
                            Ok(mut stream) => {
                                while let Some(res) = stream.next().await {
                                    match res {
                                        Ok(response_content) => {
                                            if let Err(e) = chat_tx.send(ChatStreamEvent::Chunk(response_content)) {
                                                error!("Failed to send chat chunk: {}", e);
                                            }
                                        },
                                        Err(e) => {
                                            error!("Error receiving chat chunk: {}", e);
                                            if let Err(e) = chat_tx.send(ChatStreamEvent::Error(e.to_string())) {
                                                error!("Failed to send chat stream error: {}", e);
                                            }
                                            break; 
                                        }
                                    }
                                }
                                if let Err(e) = chat_tx.send(ChatStreamEvent::Completed) {
                                    error!("Failed to send chat stream completion: {}", e);
                                }
                            },
                            Err(e) => { 
                                error!("Failed to start chat stream: {}", e);
                                if let Err(e) = chat_tx.send(ChatStreamEvent::Error(e.to_string())) {
                                   error!("Failed to send chat stream initiation error: {}", e);
                                }
                            }
                        }
                    });
                }
                app.current_input.clear();
                app.reset_cursor_position();
                app.input_mode = InputMode::Normal;
            }
        },
        KeyCode::Up | KeyCode::Down => {
            // Intercept up and down keys to prevent panic.
            // This is a temporary fix. A full implementation would handle
            // command history or multiline input navigation.
        }
        _ => handle_input_bar_key(app, key_event),
    }
}
    
fn handle_input_bar_key(app: &mut App, key_event: KeyEvent) {
    match key_event.code {
        KeyCode::Char(c) => {
            let current_byte_idx = app.current_input.char_indices()
                .nth(app.input_cursor_char_idx)
                .map_or(app.current_input.len(), |(idx, _)| idx);
            app.current_input.insert(current_byte_idx, c);
            app.input_cursor_char_idx += 1;
            app.input_bar_cursor_needs_to_be_visible = true;
        },
        KeyCode::Backspace => {
            if app.input_cursor_char_idx > 0 {
                let current_byte_idx = app.current_input.char_indices()
                    .nth(app.input_cursor_char_idx)
                    .map_or(app.current_input.len(), |(idx, _)| idx);
                
                let char_before_idx = app.current_input[..current_byte_idx].char_indices().last().map(|(idx, _)| idx);
                if let Some(idx) = char_before_idx {
                    app.current_input.remove(idx);
                    app.input_cursor_char_idx -= 1;
                }
                app.input_bar_cursor_needs_to_be_visible = true;
            }
        },
        KeyCode::Delete => {
            if app.input_cursor_char_idx < app.current_input.chars().count() {
                 let current_byte_idx = app.current_input.char_indices()
                    .nth(app.input_cursor_char_idx)
                    .map_or(app.current_input.len(), |(idx, _)| idx);

                if current_byte_idx < app.current_input.len() {
                    app.current_input.remove(current_byte_idx);
                    }
                    app.input_bar_cursor_needs_to_be_visible = true;
                }
        },
        KeyCode::Left => {
            if app.input_cursor_char_idx > 0 {
                app.input_cursor_char_idx -= 1;
                app.input_bar_cursor_needs_to_be_visible = true;
            }
        },
        KeyCode::Right => {
            if app.input_cursor_char_idx < app.current_input.chars().count() {
                app.input_cursor_char_idx += 1;
                app.input_bar_cursor_needs_to_be_visible = true;
            }
        },
        KeyCode::Home => {
            app.input_cursor_char_idx = 0;
            app.input_bar_cursor_needs_to_be_visible = true; 
        },
        KeyCode::End => {
            app.input_cursor_char_idx = app.current_input.chars().count();
            app.input_bar_cursor_needs_to_be_visible = true;
        },
        _ => {}
    }
}

fn handle_vm_wizard_mode_key(app: &mut App, key_event: KeyEvent) {
    match key_event.code {
        KeyCode::Enter => {
            let config = EnvironmentConfig {
                instance_id: app.new_vm_name.clone(),
                env_type: EnvironmentType::Vm,
                base_image: if app.new_vm_use_iso {
                    String::new()
                } else {
                    app.new_vm_source_image_path.clone()
                },
                boot_iso: if app.new_vm_use_iso {
                    Some(app.new_vm_iso_path.clone())
                } else {
                    None
                },
                cpu_cores: app.new_vm_cpu.parse().unwrap_or(1),
                memory_mb: parse_ram_str(&app.new_vm_ram_mb).unwrap_or(2048),
                disk_gb: Some(app.new_vm_disk_gb.parse().unwrap_or(20)),
                network_policy: "default".to_string(),
                security_policy: "default".to_string(),
                custom_script: None,
                template_name: None,
                labels: None,
            };

            let env_manager = Arc::clone(&app.env_manager);
            tokio::spawn(async move {
                tracing::info!("Starting VM creation for '{}'", config.instance_id);
                if let Err(e) = env_manager.lock().await.create_environment(&config) {
                    tracing::error!("Failed to create VM: {}", e);
                } else {
                    tracing::info!("Successfully started VM creation process.");
                }
            });

            app.show_new_vm_popup = false;
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Esc => {
            app.show_new_vm_popup = false;
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Tab => {
            let num_fields = 8;
            app.active_new_vm_input_idx = (app.active_new_vm_input_idx + 1) % num_fields;
        }
        KeyCode::BackTab => {
            let num_fields = 8;
            app.active_new_vm_input_idx = (app.active_new_vm_input_idx + num_fields - 1) % num_fields;
        }
        KeyCode::Char(' ') if app.active_new_vm_input_idx == 6 => {
                app.new_vm_use_iso = !app.new_vm_use_iso;
            }
        KeyCode::Char(c) => {
            match app.active_new_vm_input_idx {
            0 => app.new_vm_name.push(c),
            1 => app.new_vm_source_image_path.push(c),
            2 => app.new_vm_disk_path.push(c),
            3 => app.new_vm_cpu.push(c),
            4 => app.new_vm_ram_mb.push(c),
            5 => app.new_vm_disk_gb.push(c),
                7 if app.new_vm_use_iso => app.new_vm_iso_path.push(c),
            _ => {}
            }
        }
        KeyCode::Backspace => {
            match app.active_new_vm_input_idx {
                0 => { app.new_vm_name.pop(); },
                1 => { app.new_vm_source_image_path.pop(); },
                2 => { app.new_vm_disk_path.pop(); },
                3 => { app.new_vm_cpu.pop(); },
                4 => { app.new_vm_ram_mb.pop(); },
                5 => { app.new_vm_disk_gb.pop(); },
                7 if app.new_vm_use_iso => { app.new_vm_iso_path.pop(); },
                _ => {}
            }
        }
        _ => {}
    }
}

fn handle_confirm_destroy_mode_key(app: &mut App, key_event: KeyEvent) {
    match key_event.code {
        KeyCode::Char('y') | KeyCode::Enter => {
            if let Some(vm_name) = app.vm_to_destroy.take() {
                app.event_sender.send(AppEvent::DestroyVm(vm_name)).unwrap();
            }
            app.input_mode = InputMode::Normal;
        }
        KeyCode::Char('n') | KeyCode::Esc => {
            app.vm_to_destroy = None;
            app.input_mode = InputMode::Normal;
        },
        _ => {}
    }
} 