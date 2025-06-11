use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};

use crate::config::Config;
use crate::errors::HydraError;

// ... (keep the original content of the file, do not remove structs or methods yet)


use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

use crate::config::Config as AppConfig;

// ... (keep the original content of the file) 