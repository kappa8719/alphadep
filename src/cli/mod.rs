mod args;

use crate::cli::args::CommandLineArgs;
use crate::machine::AsyncMachine;
use crate::machine::ssh::SSHMachine;
use clap::Parser;
use interface::configuration::runtime::RuntimeConfiguration;
use interface::{
    configuration::machine::MachineConfiguration, configuration::project::ProjectConfiguration,
};
use log::info;
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
                    info!("remote/ssh: connecting -");
                    let mut machine = SSHMachine::connect(machine_configuration)
                        .await
                        .expect("failed to connect with ssh");

                    info!("remote/ssh: authenticating -");
                    machine
                        .authenticate()
                        .await
                        .expect("failed to authenticate ssh machine");

                    info!("remote/ssh: updating remote -");
                    machine.update(configuration.clone()).await.unwrap();

                    info!("remote/ssh: executing -");
                    machine.execute(configuration.clone()).await.unwrap();

                    info!("remote/ssh: closing -");
                    machine.close().await.expect("failed to close ssh");
                })
        }
    }

    Ok(())
}
