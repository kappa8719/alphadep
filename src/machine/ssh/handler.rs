use crate::machine::ssh::channel::SSHChannel;
use crate::machine::ssh::error::SSHError;
use russh::client::{Config, Handle};
use russh::keys::{PrivateKeyWithHashAlg, PublicKey};
use russh::{client, Disconnect};
use std::sync::Arc;

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

pub struct SSHHandle {
    pub handle: Handle<SSHHandler>,
}

impl SSHHandle {
    pub async fn connect(
        config: Arc<Config>,
        host: String,
        port: u16,
    ) -> Result<Self, anyhow::Error> {
        Ok(Self {
            handle: client::connect(config, (host, port), SSHHandler).await?,
        })
    }

    pub async fn authenticate_key(
        &mut self,
        user: String,
        key: PrivateKeyWithHashAlg,
    ) -> Result<(), SSHError> {
        self.handle.authenticate_publickey(user, key).await?;
        Ok(())
    }

    pub async fn authenticate_password(
        &mut self,
        user: String,
        password: String,
    ) -> Result<(), SSHError> {
        self.handle.authenticate_password(user, password).await?;
        Ok(())
    }

    pub async fn channel(&self) -> Result<SSHChannel, SSHError> {
        Ok(self.handle.channel_open_session().await?.into())
    }

    pub async fn close(&self) -> Result<(), SSHError> {
        self.handle
            .disconnect(Disconnect::ByApplication, "close called", "ENGLISH")
            .await?;
        Ok(())
    }
}
