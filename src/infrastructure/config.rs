use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroveConfig {
    #[serde(default = "default_sidebar_width_pct")]
    pub sidebar_width_pct: u16,
    #[serde(default)]
    pub projects: Vec<ProjectConfig>,
    #[serde(default)]
    pub attention_acks: Vec<WorkspaceAttentionAckConfig>,
}

const fn default_sidebar_width_pct() -> u16 {
    33
}

impl Default for GroveConfig {
    fn default() -> Self {
        Self {
            sidebar_width_pct: default_sidebar_width_pct(),
            projects: Vec::new(),
            attention_acks: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceAttentionAckConfig {
    pub workspace_path: PathBuf,
    pub marker: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    pub path: PathBuf,
    #[serde(default)]
    pub target: ProjectTarget,
    #[serde(default)]
    pub defaults: ProjectDefaults,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProjectTarget {
    #[default]
    Local,
    Remote {
        profile: String,
    },
}

impl ProjectTarget {
    pub fn badge_label(&self) -> String {
        match self {
            Self::Local => "L".to_string(),
            Self::Remote { profile } => format!("R:{profile}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectDefaults {
    #[serde(default)]
    pub base_branch: String,
    #[serde(default)]
    pub setup_commands: Vec<String>,
    #[serde(default = "default_auto_run_setup_commands")]
    pub auto_run_setup_commands: bool,
}

const fn default_auto_run_setup_commands() -> bool {
    true
}

impl Default for ProjectDefaults {
    fn default() -> Self {
        Self {
            base_branch: String::new(),
            setup_commands: Vec::new(),
            auto_run_setup_commands: default_auto_run_setup_commands(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedConfig {
    pub path: PathBuf,
    pub config: GroveConfig,
}

fn config_directory() -> Option<PathBuf> {
    if let Some(path) = dirs::config_dir() {
        return Some(path.join("grove"));
    }

    dirs::home_dir().map(|path| path.join(".config").join("grove"))
}

pub fn config_path() -> Option<PathBuf> {
    config_directory().map(|path| path.join("config.toml"))
}

pub fn load() -> Result<LoadedConfig, String> {
    let path = config_path().ok_or_else(|| "cannot resolve config path".to_string())?;
    let config = load_from_path(&path)?;
    Ok(LoadedConfig { path, config })
}

pub fn load_from_path(path: &Path) -> Result<GroveConfig, String> {
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(GroveConfig::default());
        }
        Err(error) => return Err(format!("config read failed: {error}")),
    };

    toml::from_str::<GroveConfig>(&raw).map_err(|error| format!("config parse failed: {error}"))
}

pub fn save_to_path(path: &Path, config: &GroveConfig) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Err("config path missing parent directory".to_string());
    };

    fs::create_dir_all(parent)
        .map_err(|error| format!("config directory create failed: {error}"))?;
    let encoded =
        toml::to_string_pretty(config).map_err(|error| format!("config encode failed: {error}"))?;
    fs::write(path, encoded).map_err(|error| format!("config write failed: {error}"))
}

#[cfg(test)]
mod tests;
