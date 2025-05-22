use crate::{DeploymentConfiguration, MachineConfiguration};
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct ProjectManifest {
    pub machine: MachineConfiguration,
    pub deployment: DeploymentConfiguration,
}
