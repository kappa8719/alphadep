mod args;

use crate::cli::args::CommandLineArgs;
use crate::machine::ssh::SSHMachine;
use crate::machine::AsyncMachine;
use clap::Parser;
use interface::configuration::runtime::RuntimeConfiguration;
use interface::{
    configuration::machine::MachineConfiguration, configuration::project::ProjectConfiguration,
};
use std::fs::File;
use std::io::{Read, Write};

pub fn handle() -> Result<(), ()> {
    let cli_args = CommandLineArgs::parse();

    let mut file =
        File::open("alphadep.toml").expect("alphadep.toml is required to run alphadep project");

    let mut buffer = String::new();
    file.read_to_string(&mut buffer)
        .expect("failed to read alphadep.toml");

    let configuration = toml::from_str::<ProjectConfiguration>(buffer.as_str())
        .expect("failed to parse alphadep.toml");

    if cli_args.write_archive {
        println!("writing archive -");
        let mut archive_file = File::options()
            .write(true)
            .create(true)
            .open("./alphadep-archive")
            .unwrap();

        configuration
            .deployment
            .files
            .write_archive(&mut archive_file, vec!["./alphadep-archive"])
            .unwrap();

        println!("terminating after writing archive");
        return Ok(());
    }

    match configuration.clone().machine {
        MachineConfiguration::RemoteSSH(machine_configuration) => {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    println!("remote/ssh: connecting -");
                    let mut machine = SSHMachine::connect(machine_configuration)
                        .await
                        .expect("failed to connect with ssh");

                    println!("remote/ssh: authenticating -");
                    let _ = machine
                        .authenticate()
                        .await
                        .expect("failed to authenticate ssh machine");

                    println!("remote/ssh: updating remote -");
                    machine.update(configuration.clone()).await.unwrap();

                    println!("remote/ssh: executing -");
                    machine
                        .execute(
                            configuration.clone(),
                            RuntimeConfiguration::from(configuration.clone()),
                        )
                        .await
                        .unwrap();

                    println!("remote/ssh: closing -");
                    machine.close().await.expect("failed to close ssh");
                })
        }
    }

    Ok(())
}
