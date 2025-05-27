pub mod ssh;

#[allow(dead_code)]
pub trait Machine {
    type UpdateError;
    type BuildError;
    type ExecuteError;

    fn update(&self) -> Result<(), Self::UpdateError>;
    fn build(&self) -> Result<(), Self::BuildError>;
    fn execute(&self) -> Result<(), Self::ExecuteError>;
}

#[allow(dead_code)]
pub trait AsyncMachine {
    type UpdateError;
    type BuildError;
    type ExecuteError;

    fn update(&mut self) -> impl Future<Output = Result<(), Self::UpdateError>> + Send;
    fn build(&mut self) -> impl Future<Output = Result<(), Self::BuildError>> + Send;
    fn execute(&mut self) -> impl Future<Output = Result<(), Self::ExecuteError>> + Send;
}
