use crate::ProjectManifest;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;

#[derive(Serialize, Deserialize, Debug)]
pub struct RuntimeBuildConfiguration {
    pub script: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RuntimeExecutionConfiguration {
    pub script: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RuntimeManifest {
    pub build: RuntimeBuildConfiguration,
    pub execution: RuntimeExecutionConfiguration,
    #[serde(rename = "environment-variables")]
    pub environment_variables: HashMap<String, String>,
    pub created_at: SystemTime,
}

impl From<ProjectManifest> for RuntimeManifest {
    fn from(value: ProjectManifest) -> Self {
        Self {
            build: RuntimeBuildConfiguration {
                script: value.deployment.build.script,
            },
            execution: RuntimeExecutionConfiguration {
                script: value.deployment.runtime.execute,
            },
            environment_variables: value.deployment.environment_variables,
            created_at: SystemTime::now(),
        }
    }
}