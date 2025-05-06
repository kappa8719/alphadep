use crate::Instance;
use crate::configuration::resolve_configuration;
use log::info;
use std::io::{Write, stderr};
use std::process::Command;

pub fn build(instance: &mut Instance) {
    let configuration = resolve_configuration();

    let Some(build_script) = configuration.build.script.clone() else {
        stderr()
            .write_all(b"build: abort + missing build script")
            .unwrap();
        return;
    };

    info!("build: begin -");

    let build_result = Command::new("sh")
        .arg("-c")
        .arg(build_script)
        .output()
        .unwrap();
    info!("build: exited {:?}", build_result.status.code());
}
