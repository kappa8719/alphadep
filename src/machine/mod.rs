pub mod ssh;

pub trait Machine {
    type UpdateError;
    type BuildError;
    type ExecuteError;

    fn update(&self) -> Result<(), Self::UpdateError>;
    fn build(&self) -> Result<(), Self::BuildError>;
    fn execute(&self) -> Result<(), Self::ExecuteError>;
}

pub trait AsyncMachine {
    type UpdateError;
    type BuildError;
    type ExecuteError;

    async fn update(&mut self) -> Result<(), Self::UpdateError>;
    async fn build(&mut self) -> Result<(), Self::BuildError>;
    async fn execute(&mut self) -> Result<(), Self::ExecuteError>;
}
