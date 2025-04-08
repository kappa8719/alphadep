use crate::configuration::machine::MachineConfiguration;
use crate::configuration::project::ProjectConfiguration;
use crate::machine::ssh::SSHMachine;
use crate::machine::AsyncMachine;
use russh::ChannelMsg;
use std::fs::File;
use std::io::{stdout, Read, Write};

mod configuration;
mod machine;
mod runtime;

fn main() {
    let mut file =
        File::open("alphadep.toml").expect("alphadep.toml is required to run alphadep project");

    let mut buffer = String::new();
    file.read_to_string(&mut buffer)
        .expect("Failed to read alphadep.toml");

    let configuration = toml::from_str::<ProjectConfiguration>(buffer.as_str())
        .expect("Failed to parse alphadep.toml");

    println!("alphadep specs:\n{configuration:#?}");
    println!("list: {:?}", configuration.deployment.files.list());

    let mut archive_file = File::options()
        .write(true)
        .create(true)
        .open("./alphadep-archive")
        .unwrap();

    println!("writing archive.");
    configuration
        .deployment
        .files
        .write_archive(&mut archive_file, vec!["./alphadep-archive"])
        .unwrap();

    match configuration.clone().machine {
        MachineConfiguration::RemoteSSH(machine_configuration) => {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async {
                    let mut machine = SSHMachine::connect(machine_configuration)
                        .await
                        .expect("failed to connect with ssh");

                    let result = machine
                        .authenticate()
                        .await
                        .expect("failed to authenticate ssh machine");
                    println!("ssh auth result: {result:?}");

                    println!("updating remote.");
                    machine.update(configuration.clone()).await.unwrap();

                    let mut channel = machine.handle.channel_open_session().await.unwrap();
                    channel.exec(true, "who").await.unwrap();

                    loop {
                        // There's an event available on the session channel
                        let Some(msg) = channel.wait().await else {
                            break;
                        };
                        match msg {
                            // Write data to the terminal
                            ChannelMsg::Data { ref data } => {
                                stdout().write_all(data).unwrap();
                                stdout().flush().unwrap();
                            }
                            // The command has returned an exit code
                            ChannelMsg::ExitStatus { exit_status } => {
                                println!("alphadep ssh: exited {exit_status}");
                                // cannot leave the loop immediately, there might still be more data to receive
                            }
                            _ => {}
                        }
                    }
                })
        }
    }
}
