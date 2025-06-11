// src/errors.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HydraError {
    // These variants are not constructed, so they are removed.
    // #[error("Configuration error: {0}")]
    // ConfigError(String),
    // #[error("Policy error: {0}")]
    // PolicyError(String),
    #[error("Initialization error: {component}: {message}")]
    ComponentInitError {
        component: String,
        message: String,
    },
    // #[error("CLI argument error: {0}")]
    // CliArgumentError(String),
    // #[error("TUI error: {0}")]
    // TuiError(String),
    // #[error("Environment manager error: {0}")]
    // EnvManagerError(String),
    // #[error("Session manager error: {0}")]
    // SessionManagerError(String),
    // #[error("MCP error: {0}")]
    // McpError(String),
    // #[error("SSH error: {0}")]
    // SshError(String),
    // #[error("Audit error: {0}")]
    // AuditError(String),
    // #[error("Component failure: {component}: {message}")]
    // ComponentFailure {
    //     component: String,
    //     message: String,
    // },
    // #[error("Functionality not implemented: {0}")]
    // NotImplementedError(String),
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("TOML deserialization error: {0}")]
    TomlError(#[from] toml::de::Error),

    #[error("XDG directory error: {0}")]
    XdgError(#[from] xdg::BaseDirectoriesError),

    #[error("TOML serialization error: {0}")]
    TomlSerializationError(#[from] toml::ser::Error),
    
    #[error("JSON serialization error: {0}")]
    JsonSerializationError(#[from] serde_json::Error),

    #[error("An unknown error occurred: {0}")]
    Unknown(String),
}

// Helper for converting anyhow::Error to HydraError if needed, or just use anyhow directly.
// For now, main uses anyhow::Result, so this might be less critical immediately.
impl From<anyhow::Error> for HydraError {
    fn from(err: anyhow::Error) -> Self {
        HydraError::Unknown(format!("Underlying error: {}", err))
    }
}

// You might also want specific From implementations for errors from crates like libvirt, containerd_client etc.
// when those are added, to map them into HydraError::ComponentFailure or more specific variants. 