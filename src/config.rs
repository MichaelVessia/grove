use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum MultiplexerKind {
    #[default]
    Tmux,
    Zellij,
}

impl MultiplexerKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Tmux => "tmux",
            Self::Zellij => "zellij",
        }
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
mod tests {
    use super::{GroveConfig, MultiplexerKind, ProjectConfig, load_from_path, save_to_path};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_path(label: &str) -> PathBuf {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be monotonic")
            .as_nanos();
        let pid = std::process::id();
        std::env::temp_dir().join(format!("grove-config-{label}-{pid}-{timestamp}.toml"))
    }

    #[test]
    fn missing_config_defaults_to_tmux() {
        let path = unique_temp_path("missing");
        let config = load_from_path(&path).expect("missing path should default");
        assert_eq!(
            config,
            GroveConfig {
                multiplexer: MultiplexerKind::Tmux,
                projects: Vec::new(),
            }
        );
    }

    #[test]
    fn save_and_load_round_trip() {
        let path = unique_temp_path("roundtrip");
        let config = GroveConfig {
            multiplexer: MultiplexerKind::Zellij,
            projects: vec![ProjectConfig {
                name: "grove".to_string(),
                path: PathBuf::from("/repos/grove"),
            }],
        };
        save_to_path(&path, &config).expect("config should save");

        let loaded = load_from_path(&path).expect("config should load");
        assert_eq!(loaded, config);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn load_old_config_without_projects_defaults_to_empty_projects() {
        let path = unique_temp_path("legacy");
        fs::write(&path, "multiplexer = \"tmux\"\n").expect("fixture should write");

        let loaded = load_from_path(&path).expect("legacy config should load");
        assert_eq!(loaded.projects, Vec::<ProjectConfig>::new());

        let _ = fs::remove_file(path);
    }
}
