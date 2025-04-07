use crate::configuration::deployment::DeploymentConfiguration;
use crate::configuration::machine::MachineConfiguration;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct ProjectConfiguration {
    pub machine: MachineConfiguration,
    pub deployment: DeploymentConfiguration,
}