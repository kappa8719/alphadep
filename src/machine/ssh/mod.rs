mod sftp;

use crate::{machine::AsyncMachine, runtime};
use interface::configuration::{
    deployment::DeploymentFileArchiveError,
    machine::{SSHIdentityConfiguration, SSHMachineConfiguration},
    project::ProjectConfiguration,
    runtime::RuntimeConfiguration,
};
use russh::{
    Channel, ChannelMsg, Disconnect, Preferred, client,
    client::{AuthResult, Handle, Msg},
    keys::{PrivateKeyWithHashAlg, PublicKey},
};
use russh_sftp::client::run;
use russh_sftp::protocol::FileAttributes;
use russh_sftp::{client::SftpSession, protocol::OpenFlags};
use std::fmt::format;
use std::io::stdout;
use std::path::PathBuf;
use std::{borrow::Cow, fmt::{Debug, Display, Formatter}, fs, io, sync::Arc, time::Duration};
use thiserror::Error;
use tokio::io::AsyncWriteExt;

pub struct SSHHandler;

impl client::Handler for SSHHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

pub struct SSHMachine {
    pub configuration: SSHMachineConfiguration,
    pub handle: Handle<SSHHandler>,
}

#[derive(Error, Debug)]
pub enum SSHError {
    Error(#[from] russh::Error),
    KeyError(#[from] russh::keys::Error),
    InternalError(#[from] russh::keys::ssh_key::Error),
    SftpError(#[from] russh_sftp::client::error::Error),
    UpdateError(#[from] DeploymentFileArchiveError),
    IOError(#[from] io::Error),
}

impl Display for SSHError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}

#[derive(Error, Debug)]
pub enum ExecuteError {
    #[error("remote runtime failed with status {status}")]
    RemoteRuntimeFailed { status: u32 },
}

impl SSHMachine {
    pub async fn connect(configuration: SSHMachineConfiguration) -> Result<Self, SSHError> {
        let config = Arc::new(client::Config {
            inactivity_timeout: Some(Duration::from_secs(5)),
            preferred: Preferred {
                kex: Cow::Owned(vec![
                    russh::kex::CURVE25519_PRE_RFC_8731,
                    russh::kex::EXTENSION_SUPPORT_AS_CLIENT,
                ]),
                ..Default::default()
            },
            ..Default::default()
        });
        let handle = client::connect(config, (configuration.host.clone(), 22), SSHHandler).await?;

        Ok(Self {
            configuration,
            handle: handle,
        })
    }

    pub async fn authenticate(&mut self) -> Result<AuthResult, anyhow::Error> {
        match self.configuration.identity.clone() {
            SSHIdentityConfiguration::Key { path } => {
                let key = russh::keys::load_secret_key(path, None)?;
                // let key = PrivateKey::from_bytes(fs::read(path)?.as_slice())?;
                let key = Arc::new(key);
                let key_with_hash_alg = PrivateKeyWithHashAlg::new(key, None);

                Ok(self
                    .handle
                    .authenticate_publickey(self.configuration.user.clone(), key_with_hash_alg)
                    .await?)
            }
            SSHIdentityConfiguration::Password { value } => Ok(self
                .handle
                .authenticate_password(self.configuration.user.clone(), value)
                .await?),
        }
    }

    pub fn user(&self) -> String {
        self.configuration.user.clone()
    }

    pub fn deployment_dir(&self, configuration: ProjectConfiguration) -> PathBuf {
        PathBuf::from(format!(
            "/home/{}/.alphadep/deployments/{}",
            self.user(),
            configuration.deployment.id
        ))
    }

    pub async fn acquire_channel(&self) -> Result<Channel<Msg>, anyhow::Error> {
        let channel = self.handle.channel_open_session().await?;
        Ok(channel)
    }

    pub async fn acquire_sftp(&self) -> Result<SftpSession, anyhow::Error> {
        let channel = self.acquire_channel().await?;
        channel.request_subsystem(true, "sftp").await?;
        Ok(SftpSession::new(channel.into_stream()).await?)
    }

    pub async fn close(&self) -> Result<(), anyhow::Error> {
        self.handle
            .disconnect(Disconnect::ByApplication, "close called", "")
            .await?;
        Ok(())
    }

    /// Create directories recursively on remote using sftp session
    pub async fn create_dir_all(&self, sftp: &SftpSession, path: PathBuf) -> Result<(), anyhow::Error> {
        let mut paths = vec![path];
        while let Some(parent) = paths.first().map(|p| p.parent()).flatten() {
            paths.insert(0, parent.to_path_buf());
        }

        for path in paths {
            let Some(str) = path.to_str() else {
                return Ok(());
            };

            let exists = sftp.try_exists(str).await?;
            if exists {
                let metadata = sftp.metadata(str).await?;
                if metadata.is_dir() {
                    continue;
                }
            }

            sftp.create_dir(str).await?;
        }

        Ok(())
    }

    pub fn runtime_path(&self, project: ProjectConfiguration) -> PathBuf {
        self.deployment_dir(project.clone()).join("runtime")
    }

    // pipe all stdout of exec
    async fn exec_stdout(channel: &mut Channel<Msg>) {
        loop {
            let mut stdout = tokio::io::stdout();
            if let Some(data) = channel.wait().await {
                match data {
                    ChannelMsg::Data { ref data } => {
                        stdout
                            .write_all(b"remote/ssh: exec received -- ")
                            .await
                            .unwrap();
                        stdout.write_all(data).await.unwrap();
                        stdout.flush().await.unwrap();
                    }
                    ChannelMsg::ExitStatus { exit_status } => {
                        stdout
                            .write_all(
                                format!("remote/ssh: exec exited {exit_status}\n").as_bytes(),
                            )
                            .await
                            .unwrap();
                        break;
                    }
                    _ => {}
                }
            } else {
                break;
            }
        }
    }
}

impl AsyncMachine for SSHMachine {
    type UpdateError = anyhow::Error;
    type BuildError = anyhow::Error;
    type ExecuteError = anyhow::Error;

    /// Update archive using temporary sftp tunnel
    async fn update(&mut self, project: ProjectConfiguration) -> Result<(), Self::UpdateError> {
        let sftp = self.acquire_sftp().await?;

        // create deployment directory
        self.create_dir_all(&sftp, self.deployment_dir(project.clone()))
            .await?;

        let archive_dst_path = self.deployment_dir(project.clone()).join("archive");

        let mut archive_dst = sftp
            .open_with_flags(
                archive_dst_path.to_str().unwrap_or(""),
                OpenFlags::WRITE | OpenFlags::CREATE | OpenFlags::TRUNCATE,
            )
            .await?;

        let archive_tmp_path = std::env::temp_dir()
            .join("alphadep-archive")
            .join(uuid::Uuid::new_v4().to_string());

        if let Some(parent) = archive_tmp_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        // scope to drop temporary archive file
        {
            // open temporary archive file
            let mut archive_tmp = std::fs::File::options()
                .write(true)
                .create(true)
                .open(archive_tmp_path.clone())?;

            project
                .deployment
                .files
                .write_archive(&mut archive_tmp, vec!["./alphadep-archive"])?;

            // open temporary archive file as async using tokio
            let mut archive_tmp = tokio::fs::File::open(archive_tmp_path.clone()).await?;
            tokio::io::copy(&mut archive_tmp, &mut archive_dst).await?;

            // handle drops here
        }

        // remove temporary archive file
        std::fs::remove_file(archive_tmp_path)?;

        let runtime_path = self
            .runtime_path(project.clone())
            .to_str()
            .unwrap()
            .to_string();

        // upload runtime
        let mut runtime_dst = sftp
            .open_with_flags_and_attributes(
                runtime_path.clone(),
                OpenFlags::CREATE | OpenFlags::WRITE | OpenFlags::TRUNCATE,
                FileAttributes {
                    permissions: Some(0o777),
                    ..Default::default()
                },
            )
            .await?;
        runtime_dst
            .write_all(runtime::RUNTIME_WRAPPER_BINARY)
            .await?;

        let workdir = self.deployment_dir(project.clone()).join("work");

        // create workdir
        self.create_dir_all(&sftp, workdir.clone()).await?;

        let mut channel = self.acquire_channel().await?;

        // extract
        channel
            .exec(
                true,
                format!(
                    "{r} --archive.extract {src} --archive.extract.dest {dst} --archive.extract.overwrite",
                    r = runtime_path.clone(),
                    src = archive_dst_path.to_str().unwrap(),
                    dst = workdir.to_str().unwrap()
                ),
            )
            .await?;

        Self::exec_stdout(&mut channel).await;

        let mut runtime_configuration_dst = sftp
            .open_with_flags(
                workdir.join("alphadep-runtime.toml").to_str().unwrap(),
                OpenFlags::CREATE | OpenFlags::TRUNCATE | OpenFlags::WRITE,
            )
            .await?;

        let runtime_configuration = RuntimeConfiguration::from(project.clone());
        let runtime_configuration = toml::to_string(&runtime_configuration)?;
        runtime_configuration_dst
            .write_all(runtime_configuration.as_bytes())
            .await?;

        // close session and channel used for sftp
        sftp.close().await?;

        Ok(())
    }

    async fn build(&mut self, project: ProjectConfiguration) -> Result<(), Self::BuildError> {
        todo!("build on 'target' machine is not supported yet")
    }

    async fn execute(
        &mut self,
        project: ProjectConfiguration,
        runtime: RuntimeConfiguration,
    ) -> Result<(), Self::ExecuteError> {
        let mut channel = self.acquire_channel().await?;

        let workdir = self.deployment_dir(project.clone()).join("work");
        let runtime_path = self.runtime_path(project).to_str().unwrap().to_string();

        // change to workdir
        channel
            .exec(false, format!("cd {}", workdir.to_str().unwrap()))
            .await?;

        // execute
        channel
            .exec(true, format!("{r} --execute", r = runtime_path))
            .await?;

        loop {
            match channel.wait().await.unwrap() {
                ChannelMsg::ExitStatus { exit_status } => {
                    if exit_status != 0 {
                        return Err(anyhow::Error::new(ExecuteError::RemoteRuntimeFailed {
                            status: exit_status,
                        }));
                    }

                    break;
                }
                _ => {}
            }
        }

        Ok(())
    }
}
