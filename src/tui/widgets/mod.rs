pub mod about_modal;
pub mod chat;
pub mod input_bar;
pub mod logs;
pub mod menu;
pub mod new_vm_popup;
pub mod ollama_model_list;
pub mod status_bar;
pub mod vm_list;
pub mod keybindings_modal;

#[cfg(feature = "bedrock_integration")]
pub mod bedrock_model_list;

// We can re-export widget structs here later, e.g.:
// pub use status_bar::StatusBarWidget;
// etc.


#[cfg(feature = "bedrock_integration")]
pub use self::bedrock_model_list::BedrockModelListWidget;
