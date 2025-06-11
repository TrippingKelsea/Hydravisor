// src/mcp.rs
// Model Context Protocol implementation

use serde::{Deserialize, Serialize};
// use tokio::net::{UnixListener, UnixStream}; // For Unix domain socket
// use tokio::sync::mpsc; // For message passing between MCP server and other parts of Hydravisor

// use crate::errors::HydraError; // Not used yet

// Core MCP message structure (as per mcp.design.md)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct McpMessage {
    pub r#type: String, // Using r# to allow "type" as field name. Examples: "vm/create", "model/log"
    
    // Common fields, specific commands might have more under `payload` or directly.
    // These are illustrative based on mcp.design.md examples.
    pub instance_id: Option<String>,
    pub os: Option<String>,
    pub cpu: Option<u32>,
    pub ram: Option<String>,
    pub model: Option<String>,      // e.g., "ollama:llama3"
    pub query: Option<bool>,        // For vm/state
    pub role: Option<String>,       // For vm/attach-terminal
    pub source: Option<String>,     // For model/log
    pub payload: Option<serde_json::Value>, // For generic payloads or complex types
    pub meta: Option<McpMeta>,      // Optional metadata block
    
    // Fields for envelope format (mcp.design.md)
    pub src: Option<String>,        // Source agent/entity
    pub dst: Option<String>,        // Destination VM or model
    
    // For error responses
    pub code: Option<u16>,          // e.g., 403, 503
    pub message: Option<String>,    // Error message
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct McpMeta {
    pub name: Option<String>,           // e.g., "llama-sandbox" for vm/create
    pub record_session: Option<bool>,
    // Add other meta fields as needed
}

// Since McpServer and McpClient are not used, these structs can be removed.
// pub struct McpClient { ... }
// pub struct McpServer { ... }
// pub struct McpMessageWithOrigin { ... }

// The implementation for McpServer is also unused.
// impl McpServer { ... }

// TODO: Add tests for MCP message serialization/deserialization.
// TODO: Add tests for MCP server logic (mocking client connections and core dispatcher).
// - Test client connection and disconnection.
// - Test message routing (mocked).
// - Test error handling for malformed messages or unauthorized requests. 