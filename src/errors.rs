// src/errors.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HydraError {
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Policy error: {0}")]
    PolicyError(String),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("XDG directory error: {0}")]
    XdgError(#[from] xdg::BaseDirectoriesError),

    #[error("TOML deserialization error: {0}")]
    TomlDeserializationError(#[from] toml::de::Error),

    #[error("TOML serialization error: {0}")]
    TomlSerializationError(#[from] toml::ser::Error),
    
    #[error("JSON serialization error: {0}")]
    JsonSerializationError(#[from] serde_json::Error),

    #[error("CLI argument parsing error: {0}")]
    CliArgumentError(String),

    #[error("TUI error: {0}")]
    TuiError(String),

    #[error("Environment management error: {0}")]
    EnvManagerError(String),

    #[error("Session management error: {0}")]
    SessionManagerError(String),

    #[error("MCP error: {0}")]
    McpError(String),

    #[error("SSH error: {0}")]
    SshError(String),

    #[error("Audit error: {0}")]
    AuditError(String),

    #[error("An underlying component failed: {component}: {details}")]
    ComponentFailure {
        component: String,
        details: String,
    },

    #[error("Feature not yet implemented: {0}")]
    NotImplementedError(String),

    #[error("An unknown error occurred: {0}")]
    UnknownError(String),
}

// Helper for converting anyhow::Error to HydraError if needed, or just use anyhow directly.
// For now, main uses anyhow::Result, so this might be less critical immediately.
impl From<anyhow::Error> for HydraError {
    fn from(err: anyhow::Error) -> Self {
        HydraError::UnknownError(format!("Underlying error: {}", err))
    }
}

// You might also want specific From implementations for errors from crates like libvirt, containerd_client etc.
// when those are added, to map them into HydraError::ComponentFailure or more specific variants. 