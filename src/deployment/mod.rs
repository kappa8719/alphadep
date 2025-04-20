use interface::configuration::project::ProjectConfiguration;
use std::path::PathBuf;

pub struct DeploymentSpecs {
    pub project: ProjectConfiguration,
    pub files: Vec<PathBuf>,
}
