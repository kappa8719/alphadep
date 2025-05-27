use crate::Instance;
use crate::command::ExecuteArgs;
use crate::manifest::resolve_manifest;
use log::info;
use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;
use run_script::{IoOptions, ScriptOptions};
use std::sync::{Arc, Mutex};

pub fn execute(_: &mut Instance, args: ExecuteArgs) {
    std::env::set_current_dir(args.directory).unwrap();

    let manifest = resolve_manifest();

    if !args.silent {
        info!("execution: begin -");
    }

    let child = run_script::spawn(
        manifest.execution.script.as_str(),
        &vec![],
        &ScriptOptions {
            output_redirection: IoOptions::Inherit,
            ..ScriptOptions::new()
        },
    )
    .unwrap();

    let child = Arc::new(Mutex::new(child));

    let pid = child.lock().unwrap().id();

    {
        ctrlc::set_handler(move || {
            info!("execution: killing child process before destroy");
            kill(Pid::from_raw(pid as i32), Signal::SIGINT).expect("could not kill child process");

            std::process::exit(0);
        })
        .unwrap();
    }

    let exit = child.lock().unwrap().wait().unwrap();
    if !args.silent {
        info!("execution: - exited {:?}", exit.code());
    }
}
