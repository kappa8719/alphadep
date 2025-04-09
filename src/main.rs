use crate::configuration::machine::MachineConfiguration;
use crate::configuration::project::ProjectConfiguration;
use crate::machine::AsyncMachine;
use crate::machine::ssh::SSHMachine;
use crate::runtime::RUNTIME_WRAPPER_BINARY;
use clap::Parser;
use russh::ChannelMsg;
use std::fs::File;
use std::io::{Read, Write, stdout};
use std::path::PathBuf;
use std::process::ExitCode;

mod configuration;
mod machine;
mod runtime;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct CommandLineArgs {
    /// Whether alphadep should write archive to current directory and terminate program
    #[arg(long, default_value_t = false)]
    write_archive: bool,
}

fn main() -> ExitCode {
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
        return ExitCode::SUCCESS;
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

                    println!("remote/ssh: uploading archive -");
                    machine.update(configuration.clone()).await.unwrap();

                    let mut channel = machine.handle.channel_open_session().await.unwrap();

                    loop {
                        // There's an event available on the session channel
                        let Some(msg) = channel.wait().await else {
                            break;
                        };
                        match msg {

                            ChannelMsg::Data { ref data } => {
                                stdout().write_all(data).unwrap();
                                stdout().flush().unwrap();
                            }

                            ChannelMsg::ExitStatus { exit_status } => {
                                println!("remote/ssh: exited@{exit_status}");
                            }
                            _ => {}
                        }
                    }
                })
        }
    }

    ExitCode::SUCCESS
}
