// src/mcp.rs
// Model Context Protocol implementation

use anyhow::Result;
use serde::{Deserialize, Serialize};
// use tokio::net::{UnixListener, UnixStream}; // For Unix domain socket
// use tokio::sync::mpsc; // For message passing between MCP server and other parts of Hydravisor

use crate::config::Config;
use crate::errors::HydraError;

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

// Represents an MCP client connection or context
pub struct McpClient {
    // pub client_id: String,
    // pub stream: UnixStream, // Or other transport
    // TODO: Add state related to the client, e.g. authenticated agent_id, role
}

pub struct McpServer {
    // config: McpConfig, // Derived from main Config.mcp
    // listener: UnixListener,
    // active_clients: Mutex<HashMap<String, Arc<McpClient>>>,
    // dispatcher_tx: mpsc::Sender<McpMessageWithOrigin> // To send messages to core Hydravisor logic
}

// Used to pass messages from MCP server to other parts of the application, 
// including where the message came from.
pub struct McpMessageWithOrigin {
    // pub origin_client_id: String,
    // pub message: McpMessage,
}

impl McpServer {
    pub async fn start(app_config: &Config /*, core_dispatcher_tx: mpsc::Sender<McpMessageWithOrigin>*/) -> Result<Self> {
        // TODO: Initialize McpServer based on app_config.mcp:
        // 1. Set up the Unix domain socket (or WebSocket in future) listener.
        //    - Path from app_config.mcp.socket_path.
        //    - Handle potential errors if socket path is in use or not writable.
        // 2. Initialize active_clients map.
        // 3. Store dispatcher_tx for sending messages to the core application logic.
        // 4. Start the main accept loop in a separate tokio task.
        println!("MCP Server starting. Listening on: {}, Timeout: {}ms, Heartbeat: {}s", 
                 app_config.mcp.socket_path, app_config.mcp.timeout_ms, app_config.mcp.heartbeat_interval);
        todo!("Implement McpServer start: setup listener, accept loop.");
        // Ok(McpServer { ... })
    }

    // This would run in a loop in a dedicated tokio task
    async fn accept_loop(&self) -> Result<()> {
        // loop {
        //     let (stream, _addr) = self.listener.accept().await?;
        //     let client_id = generate_unique_client_id(); // Implement this
        //     let mcp_client = Arc::new(McpClient { client_id: client_id.clone(), stream });
        //     self.active_clients.lock().unwrap().insert(client_id.clone(), Arc::clone(&mcp_client));
        //     
        //     let dispatcher_tx_clone = self.dispatcher_tx.clone();
        //     tokio::spawn(async move {
        //         if let Err(e) = Self::handle_client(mcp_client, dispatcher_tx_clone).await {
        //             eprintln!("MCP client error ({}): {}", client_id, e);
        //         }
        //         // TODO: Cleanup client from active_clients map
        //     });
        // }
        todo!("Implement MCP client accept loop.");
    }

    // Handles communication with a single connected MCP client
    async fn handle_client(_client: std::sync::Arc<McpClient>, _dispatcher_tx: tokio::sync::mpsc::Sender<McpMessageWithOrigin>) -> Result<()> {
        // loop {
        //     // 1. Read data from client.stream (e.g., length-prefixed JSON messages).
        //     // 2. Deserialize into McpMessage.
        //     // 3. Perform initial validation (e.g., presence of `type` field).
        //     // 4. TODO: Authenticate/authorize client if not already done (e.g., first message must be auth, or use transport security).
        //     //    - This involves the PolicyEngine.
        //     //    - Agent fingerprint and role would be established here.
        //     // 5. Construct McpMessageWithOrigin.
        //     // 6. Send to core Hydravisor logic via dispatcher_tx.
        //     //    - The core logic will then route it to SessionManager, EnvManager, etc.
        //     // 7. await response from core logic (if synchronous, or handle async responses).
        //     // 8. Serialize response McpMessage and send back to client.stream.
        //     // 9. Handle heartbeats (mcp/heartbeat).
        //     // 10. Handle client disconnection gracefully.
        // }
        todo!("Implement MCP client handling: read, deserialize, auth, dispatch, respond.");
    }

    pub async fn send_message_to_client(&self, _client_id: &str, _message: McpMessage) -> Result<()> {
        // TODO: Find client by client_id and send them the message.
        // This is used for Hydravisor-initiated messages to agents (e.g., model/log, mcp/authorize).
        todo!("Implement sending message to a specific MCP client.");
    }
}

// TODO: Add tests for MCP message serialization/deserialization.
// TODO: Add tests for MCP server logic (mocking client connections and core dispatcher).
// - Test client connection and disconnection.
// - Test message routing (mocked).
// - Test error handling for malformed messages or unauthorized requests. 