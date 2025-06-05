// src/ollama_manager.rs
// Manages interactions with the Ollama API

use anyhow::Result;
use crate::config::Config;
use tracing::{info, error, debug, warn}; // Added tracing macros

#[cfg(feature = "ollama_integration")]
use ollama_rs::{
    Ollama,
    models::LocalModel,
    generation::chat::{ChatMessage, ChatMessageResponse, MessageRole}, // Import MessageRole
    generation::chat::request::ChatMessageRequest,
    // ollama_rs::error::OllamaError is no longer used directly here
};

#[cfg(feature = "ollama_integration")]
use futures::stream::StreamExt;

pub struct OllamaManager {
    #[cfg(feature = "ollama_integration")]
    client: Option<Ollama>,
    // We need a way to signal that ollama is not available even if the feature is compiled
    // if the client fails to initialize.
    #[cfg(not(feature = "ollama_integration"))]
    _private: (), // Placeholder for when feature is not enabled
}

impl Default for OllamaManager {
    fn default() -> Self {
        #[cfg(feature = "ollama_integration")]
        {
            warn!("Creating default (non-functional) OllamaManager due to earlier initialization issue or feature configuration.");
            OllamaManager { client: None }
        }
        #[cfg(not(feature = "ollama_integration"))]
        {
            // This state is normal if the feature is off
            debug!("Creating default OllamaManager (ollama_integration feature disabled).");
            OllamaManager { _private: () }
        }
    }
}

#[cfg(feature = "ollama_integration")] // Helper function also needs this cfg
fn map_stream_item_error(_err: ()) -> String { // Return String instead of OllamaError
    "Error processing stream item from Ollama".to_string()
}

impl OllamaManager {
    pub fn new(app_config: &Config) -> Result<Self> {
        #[cfg(feature = "ollama_integration")]
        {
            // TODO: Allow Ollama URL to be configured in Hydravisor's config.toml
            let ollama_host = app_config.ollama_host.clone().unwrap_or_else(|| "http://localhost".to_string());
            let ollama_port = app_config.ollama_port.unwrap_or(11434);
            
            info!("Attempting to connect to Ollama at {}:{}", ollama_host, ollama_port);
            let client = Ollama::new(ollama_host, ollama_port);
            // There isn't a direct "ping" or "check connection" method in ollama-rs prior to making a real request.
            // We'll assume for now that if Ollama::new() doesn't panic, it's okay.
            // Actual availability will be checked during operations like list_models.
            info!("OllamaManager initialized for connection. Ollama integration enabled.");
            Ok(Self { client: Some(client) })
        }
        
        #[cfg(not(feature = "ollama_integration"))]
        {
            info!("OllamaManager initialized. Ollama integration NOT enabled.");
            Ok(Self { _private: () })
        }
    }

    #[cfg(feature = "ollama_integration")]
    pub async fn list_local_models(&self) -> Result<Vec<LocalModel>> {
        if let Some(client) = &self.client {
            debug!("Listing local Ollama models.");
            match client.list_local_models().await {
                Ok(models) => {
                    debug!("Successfully listed {} Ollama models.", models.len());
                    Ok(models)
                }
                Err(e) => {
                    error!("Failed to list Ollama models: {}", e);
                    Err(anyhow::anyhow!("Failed to list Ollama models: {}", e))
                }
            }
        } else {
            warn!("Ollama client not available for listing models. Returning empty list.");
            Ok(Vec::new()) 
        }
    }

    #[cfg(not(feature = "ollama_integration"))]
    #[allow(clippy::unused_async)] // To match signature, but it's not async without feature
    pub async fn list_local_models(&self) -> Result<Vec<String>> {
        // Placeholder for when ollama_integration is not enabled
        // To avoid type mismatches with callers expecting Vec<LocalModel>
        // we might need a different approach or the caller also needs to be feature-gated.
        // For now, returning a Vec<String> to make it distinct.
        info!("Ollama integration not enabled, cannot list models.");
        Ok(Vec::new())
    }

    // Placeholder for generate_response method
    #[cfg(feature = "ollama_integration")]
    pub async fn generate_response_stream(
        &self,
        model_name_param: String,
        history: Vec<crate::tui::ChatMessage>, 
        system_prompt_override: Option<String>,
    ) -> Result<impl StreamExt<Item = Result<String, String>>> { // Item error type changed to String
        if let Some(client) = &self.client {
            let mut ollama_messages: Vec<ChatMessage> = Vec::new();

            if let Some(sp) = system_prompt_override {
                if !sp.is_empty() {
                    ollama_messages.push(ChatMessage::new(MessageRole::System, sp.clone()));
                    debug!("Using system prompt override: \"{}\"", sp);
                } else {
                    debug!("System prompt override was empty, not adding system message.");
                }
            } else {
                debug!("No system prompt override provided.");
            }
            
            for tui_msg in history.iter() {
                if tui_msg.sender != "user" && tui_msg.content.is_empty() && tui_msg.thought.is_none() {
                    debug!("Skipping empty placeholder assistant message from history for model: {}", tui_msg.sender);
                    continue;
                }
                let role = if tui_msg.sender == "user" {
                    MessageRole::User
                } else {
                    MessageRole::Assistant
                };
                ollama_messages.push(ChatMessage::new(role, tui_msg.content.clone()));
            }

            if ollama_messages.is_empty() {
                error!("Cannot send empty message list to Ollama for model: {}", model_name_param);
                return Err(anyhow::anyhow!("Message list for Ollama is empty after processing history."));
            }
            if ollama_messages.last().map_or(true, |msg| msg.role != MessageRole::User) { // Compare with MessageRole enum
                 error!("The last message in the history sent to Ollama must be from the user. Model: {}", model_name_param);
                 return Err(anyhow::anyhow!("Last message to Ollama was not from User."));
            }

            debug!(
                "Sending {} messages to Ollama model: {}. Last user prompt: \"{}\"",
                ollama_messages.len(),
                model_name_param,
                ollama_messages.last().map_or("N/A", |m| m.content.as_str())
            );
            
            let chat_request = ChatMessageRequest::new(model_name_param.clone(), ollama_messages);

            match client.send_chat_messages_stream(chat_request).await {
                Ok(ollama_stream) => {
                    debug!("Successfully started chat messages stream for model: {}", model_name_param);
                    Ok(ollama_stream.map(|result_chat_message_response: Result<ChatMessageResponse, ()>| {
                        result_chat_message_response
                            .map_err(map_stream_item_error) // This now returns String
                            .map(|chat_message_response| {
                                chat_message_response.message.map_or_else(String::new, |chat_msg| chat_msg.content)
                        })
                    }))
                },
                Err(e) => {
                    error!("Failed to start chat messages stream for model {}: {}", model_name_param, e);
                    Err(anyhow::anyhow!("Failed to start chat messages stream for model {}: {}", model_name_param, e))
                }
            }
        } else {
            warn!("Ollama client not available for generation.");
             Err(anyhow::anyhow!("Ollama client not available"))
        }
    }

    #[cfg(not(feature = "ollama_integration"))]
    #[allow(clippy::unused_async)]
    pub async fn generate_response_stream(
        &self,
        model_name: String,
        history: Vec<crate::tui::ChatMessage>,
        system_prompt_override: Option<String>,
    ) -> Result<futures::stream::Empty<Result<String, String>>> { // Changed OllamaError to String for cfg-disabled case
        let last_prompt = history.last().map_or("N/A", |m| m.content.as_str());
        warn!(
            "Ollama integration not enabled. Cannot generate stream for model: {}, system_prompt: {:?}, last user prompt: {}.", 
            model_name, 
            system_prompt_override.as_deref().unwrap_or("None"),
            last_prompt
        );
        Ok(futures::stream::empty()) 
    }

    // Added to help main.rs logging logic, can be removed if OllamaManager::new becomes more robust
    // regarding feature flags and benign default states on its own.
    // For now, main.rs does the primary decision making for what is fatal.
    #[allow(dead_code)] // May not be strictly needed if Default is the main fallback
    pub fn is_functional(&self) -> bool {
        #[cfg(feature = "ollama_integration")]
        {
            self.client.is_some()
        }
        #[cfg(not(feature = "ollama_integration"))]
        {
            false // Not functional in terms of Ollama API calls if feature is off
        }
    }
} 