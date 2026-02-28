use std::ffi::OsString;
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
    #[serde(default)]
    pub launch_skip_permissions: bool,
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
            launch_skip_permissions: false,
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
    pub defaults: ProjectDefaults,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ProjectDefaults {
    #[serde(default)]
    pub base_branch: String,
    #[serde(default)]
    pub workspace_init_command: String,
    #[serde(default)]
    pub agent_env: AgentEnvDefaults,
    #[serde(default, rename = "setup_commands", skip_serializing)]
    legacy_setup_commands: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AgentEnvDefaults {
    #[serde(default)]
    pub claude: Vec<String>,
    #[serde(default)]
    pub codex: Vec<String>,
    #[serde(default)]
    pub opencode: Vec<String>,
}

impl ProjectDefaults {
    fn normalize_legacy_fields(&mut self) {
        if self.workspace_init_command.trim().is_empty() {
            let migrated = self
                .legacy_setup_commands
                .iter()
                .map(String::as_str)
                .map(str::trim)
                .find(|command| !command.is_empty())
                .unwrap_or_default()
                .to_string();
            self.workspace_init_command = migrated;
        }
        self.legacy_setup_commands.clear();
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedConfig {
    pub path: PathBuf,
    pub config: GroveConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct GlobalSettingsConfig {
    #[serde(default = "default_sidebar_width_pct")]
    sidebar_width_pct: u16,
    #[serde(default)]
    launch_skip_permissions: bool,
}

impl Default for GlobalSettingsConfig {
    fn default() -> Self {
        Self {
            sidebar_width_pct: default_sidebar_width_pct(),
            launch_skip_permissions: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
struct ProjectsStateConfig {
    #[serde(default)]
    projects: Vec<ProjectConfig>,
    #[serde(default)]
    attention_acks: Vec<WorkspaceAttentionAckConfig>,
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

pub fn projects_path() -> Option<PathBuf> {
    config_path().map(|path| projects_path_for(path.as_path()))
}

pub fn projects_path_for(config_path: &Path) -> PathBuf {
    let file_name = config_path.file_name().map_or_else(
        || OsString::from("projects.toml"),
        |name| {
            if name == "config.toml" {
                OsString::from("projects.toml")
            } else {
                let mut value = name.to_os_string();
                value.push(".projects.toml");
                value
            }
        },
    );
    config_path.with_file_name(file_name)
}

pub fn load() -> Result<LoadedConfig, String> {
    let path = config_path().ok_or_else(|| "cannot resolve config path".to_string())?;
    let config = load_from_path(&path)?;
    Ok(LoadedConfig { path, config })
}

pub fn load_from_path(path: &Path) -> Result<GroveConfig, String> {
    let settings = load_global_from_path(path)?;
    let projects = load_projects_from_path(&projects_path_for(path))?;
    Ok(GroveConfig {
        sidebar_width_pct: settings.sidebar_width_pct,
        projects: projects.projects,
        attention_acks: projects.attention_acks,
        launch_skip_permissions: settings.launch_skip_permissions,
    })
}

pub fn load_global_from_path(path: &Path) -> Result<GroveConfig, String> {
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(GroveConfig::default());
        }
        Err(error) => return Err(format!("global config read failed: {error}")),
    };

    let settings = toml::from_str::<GlobalSettingsConfig>(&raw)
        .map_err(|error| format!("global config parse failed: {error}"))?;
    Ok(GroveConfig {
        sidebar_width_pct: settings.sidebar_width_pct,
        projects: Vec::new(),
        attention_acks: Vec::new(),
        launch_skip_permissions: settings.launch_skip_permissions,
    })
}

pub fn load_projects_from_path(path: &Path) -> Result<GroveConfig, String> {
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(GroveConfig::default());
        }
        Err(error) => return Err(format!("projects config read failed: {error}")),
    };

    let mut projects = toml::from_str::<ProjectsStateConfig>(&raw)
        .map_err(|error| format!("projects config parse failed: {error}"))?;
    for project in &mut projects.projects {
        project.defaults.normalize_legacy_fields();
    }
    Ok(GroveConfig {
        sidebar_width_pct: default_sidebar_width_pct(),
        projects: projects.projects,
        attention_acks: projects.attention_acks,
        launch_skip_permissions: false,
    })
}

pub fn save_global_to_path(path: &Path, config: &GroveConfig) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Err("global config path missing parent directory".to_string());
    };

    fs::create_dir_all(parent)
        .map_err(|error| format!("global config directory create failed: {error}"))?;
    let settings = GlobalSettingsConfig {
        sidebar_width_pct: config.sidebar_width_pct,
        launch_skip_permissions: config.launch_skip_permissions,
    };
    let encoded = toml::to_string_pretty(&settings)
        .map_err(|error| format!("global config encode failed: {error}"))?;
    fs::write(path, encoded).map_err(|error| format!("global config write failed: {error}"))
}

pub fn save_projects_to_path(
    path: &Path,
    projects: &[ProjectConfig],
    attention_acks: &[WorkspaceAttentionAckConfig],
) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Err("projects config path missing parent directory".to_string());
    };

    fs::create_dir_all(parent)
        .map_err(|error| format!("projects config directory create failed: {error}"))?;
    let projects_state = ProjectsStateConfig {
        projects: projects.to_vec(),
        attention_acks: attention_acks.to_vec(),
    };
    let encoded = toml::to_string_pretty(&projects_state)
        .map_err(|error| format!("projects config encode failed: {error}"))?;
    fs::write(path, encoded).map_err(|error| format!("projects config write failed: {error}"))
}

pub fn save_projects_state_from_config_path(
    path: &Path,
    config: &GroveConfig,
) -> Result<(), String> {
    let projects_path = projects_path_for(path);
    save_projects_to_path(&projects_path, &config.projects, &config.attention_acks)
}

pub fn save_to_path(path: &Path, config: &GroveConfig) -> Result<(), String> {
    save_global_to_path(path, config)?;
    save_projects_state_from_config_path(path, config)
}

#[cfg(test)]
mod tests;
