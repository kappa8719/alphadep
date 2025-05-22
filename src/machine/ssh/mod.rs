pub mod error;
mod sftp;
mod ssh;
mod task;

use crate::{machine::AsyncMachine, runtime::RUNTIME_WRAPPER_BINARY};
use interface::{
    ProjectManifest, RuntimeManifest, SSHIdentityConfiguration, SSHMachineConfiguration,
};
use log::info;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{fs, io::Read, path::PathBuf};
use tokio::net::TcpStream;
use tokio::net::ToSocketAddrs;
use tokio::signal;

pub struct SSHMachine {
    pub project: ProjectManifest,
    pub configuration: SSHMachineConfiguration,
    pub session: ssh2::Session,
}

impl SSHMachine {
    pub async fn handshake(
        project: ProjectManifest,
        configuration: SSHMachineConfiguration,
    ) -> Result<Self, error::HandshakeError> {
        let addr = match tokio::net::lookup_host(configuration.host.clone()).await {
            Ok(mut iter) => iter.next(),
            Err(_) => tokio::net::lookup_host((configuration.host.clone(), 22))
                .await?
                .next(),
        }
        .unwrap();

        let socket = TcpStream::connect(addr)
            .await
            .map_err(|e| error::HandshakeError::IO(e))?;
        let Ok(mut session) = ssh2::Session::new() else {
            return Err(error::HandshakeError::Unknown);
        };

        session.set_tcp_stream(socket);
        session.handshake()?;

        Ok(Self {
            project,
            configuration,
            session,
        })
    }

    pub async fn authenticate(&mut self) -> Result<(), error::AuthenticateError> {
        match self.configuration.identity.clone() {
            SSHIdentityConfiguration::Key { path } => {
                let content = match fs::read_to_string(path) {
                    Ok(content) => content,
                    Err(error) => {
                        return Err(error::AuthenticateError::Key(error::KeyError::IO(error)));
                    }
                };

                Ok(self.session.userauth_pubkey_memory(
                    self.configuration.user.as_str(),
                    None,
                    content.as_str(),
                    None,
                )?)
            }
            SSHIdentityConfiguration::Password { value } => Ok(self
                .session
                .userauth_password(self.configuration.user.as_str(), value.as_str())?),
        }
    }

    pub fn deployment_dir(&self) -> PathBuf {
        PathBuf::from(format!(
            "/home/{}/.alphadep/deployments/{}",
            self.configuration.user.clone(),
            self.project.deployment.id
        ))
    }
}

impl AsyncMachine for SSHMachine {
    type UpdateError = anyhow::Error;
    type BuildError = anyhow::Error;
    type ExecuteError = anyhow::Error;

    /// Update archive using temporary sftp tunnel
    async fn update(&mut self) -> Result<(), Self::UpdateError> {
        let deployment_dir = self.deployment_dir();

        info!("update: uploading runtime");
        task::UploadRuntime {
            sftp: self.session.sftp()?,
            binary: RUNTIME_WRAPPER_BINARY,
            destination: deployment_dir.join("runtime"),
        }
        .upload()?;

        info!("update: uploading archive");
        task::UploadArchive {
            sftp: self.session.sftp()?,
            files: &self.project.deployment.files,
            destination: deployment_dir.join("archive"),
        }
        .upload()?;

        info!("update: extracting archive");
        task::ExtractArchive {
            channel: self.session.channel_session()?,
            archive: deployment_dir.join("archive"),
            destination: deployment_dir.join("work"),
            runtime: deployment_dir.join("runtime"),
        }
        .extract()?;

        info!("update: writing manifest");
        task::UploadManifest {
            sftp: self.session.sftp()?,
            destination: deployment_dir.join("work/alphadep-runtime.toml"),
            manifest: RuntimeManifest::from(self.project.clone()),
        }
        .upload()?;

        Ok(())
    }

    async fn build(&mut self) -> Result<(), Self::BuildError> {
        let workdir = self.deployment_dir().join("work");
        let runtime = self
            .deployment_dir()
            .join("runtime")
            .to_str()
            .unwrap()
            .to_string();

        let mut channel = self.session.channel_session()?;

        // execute
        channel.exec(
            format!(
                "cd {wd} && {r} build",
                wd = workdir.to_str().unwrap(),
                r = runtime
            )
            .as_str(),
        )?;

        let mut stdout = String::new();
        channel.read_to_string(&mut stdout)?;
        let stdout = stdout;

        info!(
            "remote/ssh: build finished with code {:?}\n{}\n",
            channel.exit_status(),
            stdout
                .lines()
                .map(|v| format!("remote/ssh/build: {v}"))
                .collect::<Vec<_>>()
                .join("\n")
        );

        Ok(())
    }

    async fn execute(&mut self) -> Result<(), Self::ExecuteError> {
        let workdir = self.deployment_dir().join("work");
        let runtime = self
            .deployment_dir()
            .join("runtime")
            .to_str()
            .unwrap()
            .to_string();

        let mut channel = self.session.channel_session()?;

        // execute
        info!("execution: ----------------\n");
        channel.exec(
            format!(
                "cd {wd} && {r} execute --silent",
                wd = workdir.to_str().unwrap(),
                r = runtime
            )
            .as_str(),
        )?;

        loop {
            signal::ctrl_c().await?;
            println!("ctrlc");
        }

        // {
        //     let sigint = Arc::new(AtomicBool::new(false));
        //     signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&sigint))?;
        //     while true {
        //         // if channel.eof() {
        //         //     break;
        //         // }
        //
        //         let mut buf = Vec::new();
        //         channel.read(buf.as_mut_slice())?;
        //
        //         print!("{}", String::from_utf8(buf)?);
        //     }
        // }

        info!(
            "\nexecution: ----------------\nexecution: exited {exit:?}",
            exit = channel.exit_status()
        );

        Ok(())
    }
}
