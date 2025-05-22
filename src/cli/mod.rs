mod args;

use crate::cli::args::CommandLineArgs;
use crate::machine::AsyncMachine;
use crate::machine::ssh::SSHMachine;
use clap::Parser;
use interface::{MachineConfiguration, ProjectManifest};
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

    let project = toml::from_str::<ProjectManifest>(buffer.as_str())
        .expect("failed to parse alphadep.toml");

    if cli_args.write_archive {
        println!("writing archive -");
        let mut archive_file = File::options()
            .write(true)
            .create(true)
            .open("./alphadep-archive")
            .unwrap();

        project
            .deployment
            .files
            .write_archive(&mut archive_file, vec!["./alphadep-archive"])
            .unwrap();

        println!("terminating after writing archive");
        return Ok(());
    }

    match project.clone().machine {
        MachineConfiguration::RemoteSSH(machine) => tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                info!("remote/ssh: connecting -");
                let mut machine = SSHMachine::handshake(project, machine)
                    .await
                    .expect("failed to connect with ssh");

                info!("remote/ssh: authenticating -");
                machine
                    .authenticate()
                    .await
                    .expect("failed to authenticate");

                info!("remote/ssh: updating remote -");
                machine.update().await.unwrap();

                info!("remote/ssh: executing -");
                machine.execute().await.unwrap();
            }),
    }

    Ok(())
}
