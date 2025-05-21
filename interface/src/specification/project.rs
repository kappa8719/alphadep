use crate::{DeploymentConfiguration, MachineConfiguration};
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct ProjectSpecification {
    pub machine: MachineConfiguration,
    pub deployment: DeploymentConfiguration,
}
