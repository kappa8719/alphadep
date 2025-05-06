use bon::bon;
use russh::Preferred;
use russh::keys::PublicKey;
use std::borrow::Cow;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

struct SSHHandler;
impl russh::client::Handler for SSHHandler {
    type Error = ();

    async fn check_server_key(
        &mut self,
        server_public_key: &PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

#[derive(Clone)]
pub struct SessionConfiguration {
    address: SocketAddr,
    inactivity_timeout: Option<Duration>,
    keepalive_interval: Option<Duration>,
    keepalive_max: usize,
}

#[bon]
impl SessionConfiguration {
    #[builder]
    fn new(
        address: SocketAddr,
        inactivity_timeout: Option<Duration>,
        keepalive_interval: Option<Duration>,
        keepalive_max: Option<usize>,
    ) -> SessionConfiguration {
        let keepalive_max = keepalive_max.unwrap_or(3);

        Self {
            address,
            inactivity_timeout,
            keepalive_interval,
            keepalive_max,
        }
    }
}

pub struct Session {
    handle: russh::client::Handle<SSHHandler>,
}

impl Session {
    pub async fn connect(configuration: SessionConfiguration) -> Result<Session, ()> {
        let c = configuration.clone();
        let config = Arc::new(russh::client::Config {
            inactivity_timeout: c.inactivity_timeout,
            keepalive_interval: c.keepalive_interval,
            keepalive_max: c.keepalive_max,
            preferred: Preferred {
                kex: Cow::Owned(vec![
                    russh::kex::CURVE25519_PRE_RFC_8731,
                    russh::kex::EXTENSION_SUPPORT_AS_CLIENT,
                ]),
                ..Default::default()
            },
            ..Default::default()
        });

        match russh::client::connect(config, configuration.address, SSHHandler).await {
            Ok(handle) => Ok(Self { handle }),
            Err(e) => Err(e),
        }
    }

    pub async fn acquire_channel(&self) {
        self.handle.channel_open_session().await;
    }
}
