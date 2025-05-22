use interface::ProjectManifest;
use std::path::PathBuf;

pub struct DeploymentSpecification {
    pub project: ProjectManifest,
    pub files: Vec<PathBuf>,
}
