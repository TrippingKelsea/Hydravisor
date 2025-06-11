use ratatui::style::{Color, Style, Stylize}; 

#[derive(Clone, Debug)]
pub struct AppTheme {
    pub primary_background: Color,
    pub secondary_background: Color,
    pub tertiary_background: Color,

    pub primary_foreground: Color,
    pub secondary_foreground: Color,
    pub tertiary_foreground: Color,
    pub quaternary_foreground: Color,
    
    pub border_primary: Color,
    pub border_secondary: Color,
    pub border_accent: Color,

    pub success_text: Color,

    pub list_highlight_bg: Color,
    pub list_highlight_fg: Color,

    pub status_bar_background: Color,
    pub status_bar_foreground: Color,
    pub status_bar_view_name_fg: Color,
    pub status_bar_mode_normal_bg: Color,
    pub status_bar_mode_editing_bg: Color,
    pub status_bar_mode_vm_wizard_bg: Color,
    pub status_bar_mode_confirm_destroy_bg: Color,

    pub input_bar_title: Style,

    pub vm_list_name_active: Style,
    pub vm_list_name_inactive: Style,
    pub vm_list_status_running: Style,
    pub vm_list_status_stopped: Style,
    pub vm_list_status_other: Style,

    pub ollama_model_list_name: Style,
    pub ollama_model_list_details_title: Style,

    pub chat_user_message_name: Style,
    pub chat_model_message_name: Style,
    pub chat_system_message_name: Style,

    pub log_level_trace: Style,
    pub log_level_debug: Style,
    pub log_level_info: Style,
    pub log_level_warn: Style,
    pub log_level_error: Style,

    pub popup_background: Color,
    pub popup_title: Style,
    pub popup_text: Style,
    pub popup_input_bg_active: Color,
    pub popup_input_fg_active: Color,
    pub popup_input_border_active: Color,
    pub popup_input_bg_inactive: Color,
    pub popup_input_fg_inactive: Color,
    pub popup_input_border_inactive: Color,
    pub popup_button_bg_active: Color,
    pub popup_button_fg_active: Color,
    pub popup_button_bg_inactive: Color,
    pub popup_button_fg_inactive: Color,

    pub error_text: Color,
    pub help_text: Color,
}

impl Default for AppTheme {
    fn default() -> Self {
        Self {
            primary_background: Color::Rgb(18, 18, 18), // Dark grey
            secondary_background: Color::Rgb(30, 30, 30),
            tertiary_background: Color::Rgb(50, 50, 50),

            primary_foreground: Color::Rgb(220, 220, 220), // Light grey
            secondary_foreground: Color::Rgb(160, 160, 160),
            tertiary_foreground: Color::Rgb(120, 120, 120),
            quaternary_foreground: Color::Rgb(255, 180, 0), // A standout color for special highlights

            border_primary: Color::Rgb(80, 80, 80),
            border_secondary: Color::Rgb(60, 60, 60),
            border_accent: Color::Rgb(0, 122, 204), // Bright blue

            success_text: Color::Rgb(0, 200, 0),   // Bright green

            list_highlight_bg: Color::Rgb(0, 122, 204),
            list_highlight_fg: Color::Rgb(255, 255, 255),

            status_bar_background: Color::Rgb(30, 30, 30),
            status_bar_foreground: Color::Rgb(200, 200, 200),
            status_bar_view_name_fg: Color::White,
            status_bar_mode_normal_bg: Color::Rgb(0, 122, 204), // Blue
            status_bar_mode_editing_bg: Color::Rgb(200, 50, 50), // Red
            status_bar_mode_vm_wizard_bg: Color::Rgb(100, 60, 200), // Purple
            status_bar_mode_confirm_destroy_bg: Color::Rgb(255, 100, 0), // Orange

            input_bar_title: Style::default().fg(Color::Rgb(0, 150, 255)),

            vm_list_name_active: Style::default().fg(Color::White).bold(),
            vm_list_name_inactive: Style::default().fg(Color::Rgb(160, 160, 160)),
            vm_list_status_running: Style::default().fg(Color::Rgb(0, 220, 0)),
            vm_list_status_stopped: Style::default().fg(Color::Rgb(220, 80, 80)),
            vm_list_status_other: Style::default().fg(Color::Rgb(255, 180, 0)),

            ollama_model_list_name: Style::default().fg(Color::White),
            ollama_model_list_details_title: Style::default().fg(Color::Rgb(120, 120, 120)),

            chat_user_message_name: Style::default().fg(Color::Rgb(0, 180, 220)).bold(),
            chat_model_message_name: Style::default().fg(Color::Rgb(150, 150, 255)).bold(),
            chat_system_message_name: Style::default().fg(Color::Rgb(255, 120, 120)).bold(),

            log_level_trace: Style::default().fg(Color::Rgb(120, 120, 120)),
            log_level_debug: Style::default().fg(Color::Rgb(150, 150, 255)),
            log_level_info: Style::default().fg(Color::Rgb(0, 200, 0)),
            log_level_warn: Style::default().fg(Color::Rgb(255, 180, 0)),
            log_level_error: Style::default().fg(Color::Rgb(255, 50, 50)).bold(),
            
            popup_background: Color::Rgb(45, 45, 45),
            popup_title: Style::default().fg(Color::White).bold(),
            popup_text: Style::default().fg(Color::Rgb(200, 200, 200)),
            popup_input_bg_active: Color::Rgb(0, 0, 0),
            popup_input_fg_active: Color::White,
            popup_input_border_active: Color::Rgb(0, 122, 204),
            popup_input_bg_inactive: Color::Rgb(30, 30, 30),
            popup_input_fg_inactive: Color::Rgb(160, 160, 160),
            popup_input_border_inactive: Color::Rgb(80, 80, 80),
            popup_button_bg_active: Color::Rgb(0, 122, 204),
            popup_button_fg_active: Color::White,
            popup_button_bg_inactive: Color::Rgb(80, 80, 80),
            popup_button_fg_inactive: Color::Rgb(30, 30, 30),

            error_text: Color::Rgb(255, 50, 50),
            help_text: Color::Rgb(120, 120, 120),
        }
    }
} 