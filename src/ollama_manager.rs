// src/ollama_manager.rs
// Manages interactions with the Ollama API

use anyhow::Result;
use crate::config::Config;
use tracing::{info, error, debug, warn}; // Added tracing macros

#[cfg(feature = "ollama_integration")]
use ollama_rs::Ollama;

#[cfg(feature = "ollama_integration")]
use ollama_rs::models::LocalModel;

#[cfg(feature = "ollama_integration")]
use ollama_rs::generation::completion::request::GenerationRequest;

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
        model_name: String,
        prompt: String,
    ) -> Result<impl StreamExt<Item = Result<String, ollama_rs::error::OllamaError>>> {
        if let Some(client) = &self.client {
            debug!("Generating response stream from Ollama model: {}, prompt: \"{}\"", model_name, prompt);
            let request = GenerationRequest::new(model_name.clone(), prompt.clone()); // Clone for potential re-use or logging
            match client.generate_stream(request).await {
                Ok(stream) => {
                    debug!("Successfully started generation stream for model: {}", model_name);
                    Ok(stream.map(|res_chunk_result| {
                        // res_chunk_result is Result<Vec<GenerationResponse>, OllamaError>
                        res_chunk_result.map(|generation_responses| {
                            // Concatenate responses from the Vec<GenerationResponse>
                            // Typically, there's one response string per GenerationResponse object in the vector.
                            generation_responses.into_iter().map(|gr| gr.response).collect::<String>()
                        })
                    }))
                },
                Err(e) => {
                    error!("Failed to start generation stream for model {}: {}", model_name, e);
                    Err(anyhow::anyhow!("Failed to start generation stream for model {}: {}", model_name, e))
                }
            }
        } else {
            warn!("Ollama client not available for generation.");
            // This case is tricky for streams. We might need to return an empty stream or an error that the caller can handle.
            // For now, let's use a simple error.
             Err(anyhow::anyhow!("Ollama client not available"))
        }
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