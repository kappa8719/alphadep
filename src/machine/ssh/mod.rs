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
use std::sync::atomic::AtomicBool;
use std::{fs, io::Read, path::PathBuf};
use tokio::net::TcpStream;

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
        let addr = match match tokio::net::lookup_host(configuration.host.clone()).await {
            Ok(mut iter) => iter.next(),
            Err(_) => tokio::net::lookup_host((configuration.host.clone(), 22))
                .await?
                .next(),
        } {
            None => return Err(error::HandshakeError::Lookup),
            Some(addr) => addr,
        };

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

        // loop {
        //     channel.request_pty("xterm", None, None)?;
        //     channel.handle_extended_data(ssh2::ExtendedData::Merge)?;
        //     channel.shell()?;
        //
        //     let stdout = std::io::stdout();
        //     let mut stdout = stdout;
        //     let mut stdin = std::io::stdin();
        //
        //     let mut buff_in = Vec::new();
        //     while !channel.eof() {
        //         let bytes_available = channel.read_window().available;
        //         if bytes_available > 0 {
        //             let mut buffer = vec![0; bytes_available as usize];
        //             channel.read_exact(&mut buffer)?;
        //             let _ = stdout.write(&buffer);
        //             let _ = stdout.flush();
        //         }
        //
        //         // Using async_stdin to avoid blocking, should this also respect the WriteWindow?
        //         stdin.read(&mut buff_in)?;
        //         let _ = channel.write(&buff_in);
        //         buff_in.clear();
        //     }
        //     channel.wait_close()?;
        // }

        // execute
        info!("execution: ----------------");
        info!("");
        channel.exec(
            format!(
                "{r} execute --directory {wd}",
                wd = workdir.to_str().unwrap(),
                r = runtime
            )
            .as_str(),
        )?;

        unsafe {
            let sigint = Arc::new(AtomicBool::new(false));
            signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&sigint))?;

            loop {
                let locked = channel.lock();

                let mut buffer = [0u8; 32];
                let bytes = libssh2_sys::libssh2_channel_read_ex(
                    locked.raw,
                    0 as std::ffi::c_int,
                    buffer.as_mut_ptr() as *mut _,
                    buffer.len(),
                );

                if bytes > 0 {
                    println!("read {bytes} bytes");
                }
            }
        }

        if channel.read_window().available > 0 {
            info!("available");
            let mut buf = String::new();
            channel.read_to_string(&mut buf)?;
            info!("{}", buf);
        }

        channel.wait_eof()?;
        channel.wait_close()?;

        info!("execution: ----------------");
        info!("execution: exited {exit:?}", exit = channel.exit_status());

        Ok(())
    }
}
