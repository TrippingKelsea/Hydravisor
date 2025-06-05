use ratatui::style::{Color, Modifier, Style}; 

#[derive(Debug, Clone)]
pub struct AppTheme {
    // General
    pub primary_background: Color,
    pub primary_foreground: Color,
    pub secondary_foreground: Color,
    pub tertiary_foreground: Color,
    pub border_primary: Color,
    pub border_secondary: Color,
    pub border_accent: Color,
    pub highlight_style: Style, // For list selections, etc.
    pub error_text: Color,
    pub warning_text: Color,
    pub success_text: Color,
    pub info_text: Color,

    // Status Bar
    pub status_bar_background: Color,
    pub status_bar_foreground: Color,
    pub status_bar_mode_normal_bg: Color,
    pub status_bar_mode_editing_bg: Color,
    pub status_bar_mode_vm_wizard_bg: Color,
    pub status_bar_mode_confirm_destroy_bg: Color,
    pub status_bar_view_name_fg: Color,

    // Input Bar
    pub input_bar_background: Color,
    pub input_bar_text_fg: Color,
    pub input_bar_border: Color,
    pub input_bar_title: Style,
    pub input_bar_text: Color,

    // VM List
    pub vm_list_title: Style,
    pub vm_state_running: Color,
    pub vm_state_stopped: Color,
    pub vm_state_suspended: Color,
    pub vm_state_other: Color,

    // Ollama Model List
    pub ollama_list_title: Style,
    pub ollama_model_details_label: Style,
    pub ollama_model_details_value: Style,
    pub ollama_system_prompt_label: Style,
    pub ollama_system_prompt_tag: Style,
    pub ollama_system_prompt_content: Style,

    // Chat Widget
    pub chat_title: Style,
    pub chat_info_text: Color,
    pub chat_user_sender: Style,
    pub chat_model_sender: Style,
    pub chat_thought_style: Style,
    pub chat_timestamp: Style,
    pub chat_streaming_indicator: Color, // For the "..." or similar
    pub chat_model_content_style: Style,       // New: For model's main text content
    pub chat_model_content_use_background: bool, // New: Toggle for model text background

    // Log View
    pub log_title: Style,
    pub log_level_trace: Style,
    pub log_level_debug: Style,
    pub log_level_info: Style,
    pub log_level_warn: Style,
    pub log_level_error: Style,
    pub log_timestamp: Style,
    pub log_target: Style,

    // Popup
    pub popup_border: Color,
    pub popup_background: Color,
}

impl Default for AppTheme {
    fn default() -> Self {
        Self {
            // General
            primary_background: Color::Reset, // Often means terminal default
            primary_foreground: Color::White,
            secondary_foreground: Color::Gray, 
            tertiary_foreground: Color::DarkGray,
            border_primary: Color::DarkGray,
            border_secondary: Color::LightCyan, // Example for active borders
            border_accent: Color::Cyan,
            highlight_style: Style::default(), // New: No visual change for highlight
            error_text: Color::Red,
            warning_text: Color::Yellow,
            success_text: Color::Green,
            info_text: Color::Cyan,

            // Status Bar
            status_bar_background: Color::Blue,
            status_bar_foreground: Color::White,
            status_bar_mode_normal_bg: Color::LightCyan,
            status_bar_mode_editing_bg: Color::LightMagenta,
            status_bar_mode_vm_wizard_bg: Color::LightGreen,
            status_bar_mode_confirm_destroy_bg: Color::LightRed,
            status_bar_view_name_fg: Color::Yellow,

            // Input Bar
            input_bar_background: Color::DarkGray,
            input_bar_text_fg: Color::White,
            input_bar_border: Color::White,
            input_bar_title: Style::default().fg(Color::Rgb(180, 180, 180)),
            input_bar_text: Color::Rgb(220, 220, 220),

            // VM List
            vm_list_title: Style::default().fg(Color::LightCyan).add_modifier(Modifier::BOLD),
            vm_state_running: Color::Green,
            vm_state_stopped: Color::Red,
            vm_state_suspended: Color::Yellow,
            vm_state_other: Color::Gray,

            // Ollama Model List
            ollama_list_title: Style::default().fg(Color::LightGreen).add_modifier(Modifier::BOLD),
            ollama_model_details_label: Style::default().fg(Color::Gray),
            ollama_model_details_value: Style::default().fg(Color::White),
            ollama_system_prompt_label: Style::default().fg(Color::Gray),
            ollama_system_prompt_tag: Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
            ollama_system_prompt_content: Style::default().fg(Color::White),

            // Chat Widget
            chat_title: Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
            chat_info_text: Color::LightBlue,
            chat_user_sender: Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            chat_model_sender: Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
            chat_thought_style: Style::default().fg(Color::Yellow).add_modifier(Modifier::ITALIC),
            chat_timestamp: Style::default().fg(Color::DarkGray), // Keep it subtle
            chat_streaming_indicator: Color::LightYellow,
            chat_model_content_style: Style::default().fg(Color::Rgb(220, 220, 220)), // Default: Light gray text, no background
            chat_model_content_use_background: false, // Default: No background for model text

            // Log View
            log_title: Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            log_level_trace: Style::default().fg(Color::Magenta), // Adjusted from previous direct color
            log_level_debug: Style::default().fg(Color::Blue),
            log_level_info: Style::default().fg(Color::Green),
            log_level_warn: Style::default().fg(Color::Yellow),
            log_level_error: Style::default().fg(Color::Red),
            log_timestamp: Style::default().fg(Color::DarkGray),
            log_target: Style::default().fg(Color::Cyan),
            
            // Popup
            popup_border: Color::Yellow,
            popup_background: Color::DarkGray,
        }
    }
} 