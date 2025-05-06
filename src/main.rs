use std::process::ExitCode;

mod cli;
mod machine;
mod runtime;
mod util;

fn main() -> ExitCode {
    colog::init();
    ctrlc::set_handler(move || {
        std::process::exit(0);
    }).unwrap();

    match cli::handle() {
        Ok(_) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}
