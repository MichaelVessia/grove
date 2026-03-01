use crate::application::services::discovery_service::bootstrap_data_for_projects as discover_bootstrap_for_projects;
use crate::infrastructure::adapters::BootstrapData;
use crate::infrastructure::config::ProjectConfig;

pub(super) fn bootstrap_data_for_projects(projects: &[ProjectConfig]) -> BootstrapData {
    discover_bootstrap_for_projects(projects)
}
