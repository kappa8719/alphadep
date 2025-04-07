use crate::configuration::project::ProjectConfiguration;
use crate::configuration::runtime::RuntimeConfiguration;
use serde::Serialize;

pub mod ssh;

pub trait Machine {
    type UpdateError;
    type BuildError;
    type ExecuteError;

    fn update(&self, project: ProjectConfiguration) -> Result<(), Self::UpdateError>;
    fn build(&self, project: ProjectConfiguration) -> Result<(), Self::BuildError>;
    fn execute(&self, runtime: RuntimeConfiguration) -> Result<(), Self::ExecuteError>;
}

pub trait AsyncMachine {
    type UpdateError;
    type BuildError;
    type ExecuteError;

    async fn update(&self, project: ProjectConfiguration) -> Result<(), Self::UpdateError>;
    async fn build(&self, project: ProjectConfiguration) -> Result<(), Self::BuildError>;
    async fn execute(&self, runtime: RuntimeConfiguration) -> Result<(), Self::ExecuteError>;
}