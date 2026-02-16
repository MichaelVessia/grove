use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum MultiplexerKind {
    #[default]
    Tmux,
}

impl MultiplexerKind {
    pub fn label(self) -> &'static str {
        "tmux"
    }

    pub const fn next(self) -> Self {
        self
    }

    pub const fn previous(self) -> Self {
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct GroveConfig {
    #[serde(default)]
    pub multiplexer: MultiplexerKind,
    #[serde(default)]
    pub projects: Vec<ProjectConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    pub path: PathBuf,
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
