extern crate core;

use std::process::ExitCode;

mod cli;
mod machine;
mod runtime;
mod util;

fn main() -> ExitCode {
    colog::init();

    match cli::handle() {
        Ok(_) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}
