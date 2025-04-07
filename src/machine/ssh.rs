use crate::configuration::deployment::DeploymentFileArchiveError;
use crate::configuration::machine::{SSHIdentityConfiguration, SSHMachineConfiguration};
use crate::configuration::project::ProjectConfiguration;
use crate::configuration::runtime::RuntimeConfiguration;
use crate::machine::{AsyncMachine, Machine};
use russh::client::{AuthResult, Handle};
use russh::keys::{PrivateKeyWithHashAlg, PublicKey};
use russh::{Preferred, client};
use russh_sftp::client::SftpSession;
use russh_sftp::protocol::OpenFlags;
use std::borrow::Cow;
use std::fmt::{Debug, Display, Formatter};
use std::io;
use std::io::Write;
use std::sync::Arc;
use std::time::Duration;
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
            handle,
        })
    }

    pub async fn authenticate(&mut self) -> Result<AuthResult, SSHError> {
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

    fn build_archive(project_configuration: ProjectConfiguration) {}
}

impl AsyncMachine for SSHMachine {
    type UpdateError = SSHError;
    type BuildError = ();
    type ExecuteError = ();

    async fn update(&self, project: ProjectConfiguration) -> Result<(), Self::UpdateError> {
        let channel = self.handle.channel_open_session().await?;
        channel.request_subsystem(true, "sftp").await.unwrap();

        let sftp = SftpSession::new(channel.into_stream()).await.unwrap();
        let mut archive_dst = sftp
            .open_with_flags(
                "alphadep-archive",
                OpenFlags::WRITE | OpenFlags::CREATE | OpenFlags::TRUNCATE,
            )
            .await?;

        let archive_tmp_path = std::env::temp_dir()
            .join("alphadep-archive")
            .join(uuid::Uuid::new_v4().to_string());

        if let Some(parent) = archive_tmp_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let mut archive_tmp = std::fs::File::options()
            .write(true)
            .create(true)
            .open(archive_tmp_path.clone())?;

        project
            .deployment
            .files
            .write_archive(&mut archive_tmp, vec!["./alphadep-archive"])?;

        let mut archive_tmp = tokio::fs::File::open(archive_tmp_path).await?;

        tokio::io::copy(&mut archive_tmp, &mut archive_dst).await?;

        Ok(())
    }

    async fn build(&self, project: ProjectConfiguration) -> Result<(), Self::BuildError> {
        todo!("build on 'target' machine is not supported yet")
    }

    async fn execute(&self, runtime: RuntimeConfiguration) -> Result<(), Self::ExecuteError> {
        Ok(())
    }
}
