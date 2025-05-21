use interface::ProjectSpecification;
use std::path::PathBuf;

pub struct DeploymentSpecification {
    pub project: ProjectSpecification,
    pub files: Vec<PathBuf>,
}
