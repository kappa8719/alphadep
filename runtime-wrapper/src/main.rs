use alphadep::configuration::runtime::RuntimeConfiguration;
use std::fs;
use std::process::Command;

fn main() {
    let configuration = fs::read_to_string("alphadep-runtime.toml").unwrap();
    let configuration = toml::from_str::<RuntimeConfiguration>(configuration.as_str()).unwrap();

    if let Some(build_script) = configuration.build.script.clone() {
        let build_result = Command::new("sh")
            .arg("-c")
            .arg(build_script)
            .output()
            .unwrap();
        println!(
            "build result {:?}\n{}",
            build_result.status.code(),
            String::from_utf8(build_result.stdout).unwrap().as_str()
        );
    }

    let exec_result = Command::new("sh")
        .arg("-c")
        .arg(configuration.execution.script.clone())
        .output()
        .unwrap();
    println!(
        "exec result {:?}\n{}",
        exec_result.status.code(),
        String::from_utf8(exec_result.stdout).unwrap().as_str()
    );

    println!("Parsed\n{:#?}", configuration);
}
