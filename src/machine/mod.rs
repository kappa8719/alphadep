use interface::configuration::{project::ProjectConfiguration, runtime::RuntimeConfiguration};

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

    async fn update(&mut self, project: ProjectConfiguration) -> Result<(), Self::UpdateError>;
    async fn build(&mut self, project: ProjectConfiguration) -> Result<(), Self::BuildError>;
    async fn execute(
        &mut self,
        project: ProjectConfiguration
    ) -> Result<(), Self::ExecuteError>;
}
