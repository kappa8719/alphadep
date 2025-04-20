use std::process::ExitCode;

mod cli;
mod machine;
mod runtime;

fn main() -> ExitCode {
    match cli::handle() {
        Ok(_) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}
