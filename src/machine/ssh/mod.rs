pub mod channel;
pub mod error;
pub mod handler;
pub mod sftp;

use crate::machine::ssh::channel::SSHExecution;
use crate::machine::ssh::handler::SSHHandle;
use crate::{
    machine::AsyncMachine,
    machine::ssh::sftp::{Sftp, SftpOpenOptions},
    runtime,
};
use interface::configuration::{
    machine::{SSHIdentityConfiguration, SSHMachineConfiguration},
    project::ProjectConfiguration,
    runtime::RuntimeConfiguration,
};
use log::info;
use russh::{Channel, ChannelMsg, Preferred, client, client::Msg, keys::PrivateKeyWithHashAlg};
use russh_sftp::{client::SftpSession, protocol::FileAttributes, protocol::OpenFlags};
use std::path::PathBuf;
use std::{
    borrow::Cow,
    fmt::{Debug, Display},
    sync::Arc,
    time::Duration,
};
use tokio::io::AsyncWriteExt;

pub struct SSHMachine {
    pub configuration: SSHMachineConfiguration,
    pub handle: SSHHandle,
}

impl SSHMachine {
    pub async fn connect(configuration: SSHMachineConfiguration) -> Result<Self, anyhow::Error> {
        let config = Arc::new(client::Config {
            inactivity_timeout: None,
            preferred: Preferred {
                kex: Cow::Owned(vec![
                    russh::kex::CURVE25519_PRE_RFC_8731,
                    russh::kex::EXTENSION_SUPPORT_AS_CLIENT,
                ]),
                ..Default::default()
            },
            ..Default::default()
        });
        let handle = SSHHandle::connect(config, configuration.host.clone(), 22).await?;

        Ok(Self {
            configuration,
            handle,
        })
    }

    pub async fn authenticate(&mut self) -> Result<(), anyhow::Error> {
        match self.configuration.identity.clone() {
            SSHIdentityConfiguration::Key { path } => {
                let key = russh::keys::load_secret_key(path, None)?;
                // let key = PrivateKey::from_bytes(fs::read(path)?.as_slice())?;
                let key = Arc::new(key);
                let key_with_hash_alg = PrivateKeyWithHashAlg::new(key, None);

                Ok(self
                    .handle
                    .authenticate_key(self.configuration.user.clone(), key_with_hash_alg)
                    .await?)
            }
            SSHIdentityConfiguration::Password { value } => Ok(self
                .handle
                .authenticate_password(self.configuration.user.clone(), value)
                .await?),
        }
    }

    pub async fn acquire_sftp(&self) -> Result<Sftp, anyhow::Error> {
        let channel = self.handle.handle.channel_open_session().await?;
        channel.request_subsystem(true, "sftp").await?;
        let session = SftpSession::new(channel.into_stream()).await?;

        Ok(Sftp::new(session))
    }

    pub async fn close(&self) -> Result<(), anyhow::Error> {
        self.handle.close().await?;
        Ok(())
    }

    pub fn runtime_path(&self, project: ProjectConfiguration) -> PathBuf {
        self.deployment_dir(project.clone()).join("runtime")
    }

    pub fn deployment_dir(&self, configuration: ProjectConfiguration) -> PathBuf {
        PathBuf::from(format!(
            "/home/{}/.alphadep/deployments/{}",
            self.configuration.user.clone(),
            configuration.deployment.id
        ))
    }
}

impl AsyncMachine for SSHMachine {
    type UpdateError = anyhow::Error;
    type BuildError = anyhow::Error;
    type ExecuteError = anyhow::Error;

    /// Update archive using temporary sftp tunnel
    async fn update(&mut self, project: ProjectConfiguration) -> Result<(), Self::UpdateError> {
        let sftp = self.acquire_sftp().await.unwrap();

        // create deployment directory
        sftp.create_dir_all(self.deployment_dir(project.clone()))
            .await
            .unwrap();

        let archive_dst_path = self.deployment_dir(project.clone()).join("archive");

        let mut archive_dst = sftp
            .open_with_options(
                archive_dst_path.to_str().unwrap_or(""),
                SftpOpenOptions::from_flags(
                    OpenFlags::WRITE | OpenFlags::CREATE | OpenFlags::TRUNCATE,
                ),
            )
            .await
            .unwrap();

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
                .open(archive_tmp_path.clone())
                .unwrap();

            project
                .deployment
                .files
                .write_archive(&mut archive_tmp, vec!["./alphadep-archive"])?;

            // open temporary archive file as async using tokio
            let mut archive_tmp = tokio::fs::File::open(archive_tmp_path.clone())
                .await
                .unwrap();
            tokio::io::copy(&mut archive_tmp, &mut archive_dst)
                .await
                .unwrap();

            // handle drops here
        }

        // remove temporary archive file
        std::fs::remove_file(archive_tmp_path).unwrap();

        let runtime_path = self
            .runtime_path(project.clone())
            .to_str()
            .unwrap()
            .to_string();

        // upload runtime
        let mut runtime_dst = sftp
            .open_with_options(
                runtime_path.clone(),
                SftpOpenOptions::from_flags(
                    OpenFlags::CREATE | OpenFlags::WRITE | OpenFlags::TRUNCATE,
                )
                .with_attributes(FileAttributes {
                    permissions: Some(0o700),
                    ..Default::default()
                }),
            )
            .await
            .unwrap();
        runtime_dst
            .write_all(runtime::RUNTIME_WRAPPER_BINARY)
            .await
            .unwrap();

        let workdir = self.deployment_dir(project.clone()).join("work");

        // create workdir
        sftp.create_dir_all(workdir.clone()).await.unwrap();
        sftp.close().await.unwrap();

        let mut channel = self.handle.channel().await.unwrap();

        // extract
        let result = channel
            .execute(format!(
                "cd {workdir} && {r} extract --archive {src} --destination {dst}",
                workdir = workdir.to_str().unwrap(),
                r = runtime_path.clone(),
                src = archive_dst_path.to_str().unwrap(),
                dst = workdir.to_str().unwrap()
            ))
            .await
            .unwrap();
        info!("extract finished with code {:?}", result.exit);

        let sftp = self.acquire_sftp().await.unwrap();
        let mut runtime_configuration_dst = sftp
            .open_with_options(
                workdir.join("alphadep-runtime.toml").to_str().unwrap(),
                SftpOpenOptions::from_flags(
                    OpenFlags::CREATE | OpenFlags::TRUNCATE | OpenFlags::WRITE,
                ),
            )
            .await
            .unwrap();

        let runtime_configuration = RuntimeConfiguration::from(project.clone());
        let runtime_configuration = toml::to_string(&runtime_configuration).unwrap();
        runtime_configuration_dst
            .write_all(runtime_configuration.as_bytes())
            .await
            .unwrap();

        sftp.close().await.unwrap();

        Ok(())
    }

    async fn build(&mut self, project: ProjectConfiguration) -> Result<(), Self::BuildError> {
        let mut channel = self.handle.channel().await?;

        let workdir = self.deployment_dir(project.clone()).join("work");
        let runtime = self
            .deployment_dir(project.clone())
            .join("runtime")
            .to_str()
            .unwrap()
            .to_string();

        // execute
        let result = channel
            .execute(format!(
                "cd {wd} && {r} build",
                wd = workdir.to_str().unwrap(),
                r = runtime
            ))
            .await?;
        info!(
            "remote/ssh: build finished with code {:?}\n{}\n",
            result.exit,
            result
                .data
                .lines()
                .map(|v| format!("remote/ssh/build: {v}"))
                .collect::<Vec<_>>()
                .join("\n")
        );

        Ok(())
    }

    async fn execute(&mut self, project: ProjectConfiguration) -> Result<(), Self::ExecuteError> {
        let mut channel = self.handle.channel().await?;

        let workdir = self.deployment_dir(project.clone()).join("work");
        let runtime = self
            .deployment_dir(project.clone())
            .join("runtime")
            .to_str()
            .unwrap()
            .to_string();

        // execute
        info!("execution: ----------------\n");
        let result = channel
            .execute(
                SSHExecution::from(format!(
                    "cd {wd} && {r} execute --silent",
                    wd = workdir.to_str().unwrap(),
                    r = runtime
                ))
                .with_redirected_output(),
            )
            .await?;

        info!(
            "\nexecution: ----------------\nexecution: exited {exit:?}",
            exit = result.exit
        );

        Ok(())
    }
}
