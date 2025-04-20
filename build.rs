use shadow_rs::{BuildPattern, ShadowBuilder};
use std::process::{Command, Stdio};

const ENV_NO_SUBMODULE_BUILD: &str = "ALPHADEP_NO_SUBMODULE_BUILD";

fn main() {
    println!("cargo::warning=building");
    println!("cargo::rerun-if-changed=build.rs");

    return;
    // skip submodule build if current module is submodule or flag is set
    let no_submodule_build = match std::env::var(ENV_NO_SUBMODULE_BUILD) {
        Ok(v) => v == "1" || v.to_lowercase() == "true",
        Err(_) => false,
    } || env!("CARGO_PKG_NAME") == "runtime";

    // do submodule build
    if !no_submodule_build {
        println!("cargo::warning=building submodule 'runtime'");
        let _ = Command::new("cargo")
            .args(&["build", "--release", "-p", "runtime"])
            .env(ENV_NO_SUBMODULE_BUILD, "1") // Prevent recursive submodule build
            .stdout(Stdio::null())
            .output();
        println!("cargo::warning=built submodule 'runtime'");
    }
}