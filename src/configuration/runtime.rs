use crate::configuration::project::ProjectConfiguration;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug)]
pub struct RuntimeBuildConfiguration {
    pub script: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RuntimeExecutionConfiguration {
    pub script: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RuntimeConfiguration {
    pub build: RuntimeBuildConfiguration,
    pub execution: RuntimeExecutionConfiguration,
    #[serde(rename = "environment-variables")]
    pub environment_variables: HashMap<String, String>,
}

impl From<ProjectConfiguration> for RuntimeConfiguration {
    fn from(value: ProjectConfiguration) -> Self {
        Self {
            build: RuntimeBuildConfiguration {
                script: value.deployment.build.script,
            },
            execution: RuntimeExecutionConfiguration {
                script: value.deployment.runtime.execute,
            },
            environment_variables: value.deployment.environment_variables,
        }
    }
}
