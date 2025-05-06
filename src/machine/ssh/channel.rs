use crate::machine::ssh::error::SSHError;
use log::info;
use russh::client::Msg;
use russh::keys::agent::server::ServerError::E;
use russh::{Channel, ChannelMsg, Sig};
use std::io;
use std::io::Write;
use std::ops::Add;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::time::Instant;


pub struct SSHChannel {
    pub inner: Channel<Msg>,
    execution: Option<SSHExecution>,
}

impl From<Channel<Msg>> for SSHChannel {
    fn from(value: Channel<Msg>) -> Self {
        Self {
            inner: value,
            execution: None,
        }
    }
}

impl Drop for SSHChannel {
    fn drop(&mut self) {
        info!("dropping ssh channel");
        futures::executor::block_on(async {
            if let Some(execution) = self.execution.clone() {
                info!("sending SIGINT to ssh channel with ongoing execution");
                io::stdout().flush().unwrap();
                self.inner
                    .signal(Sig::INT)
                    .await
                    .expect("could not send SIGINT before dropping ssh channel");
                if execution.hold_until_end {
                    self.handle_execute_receives(execution.deadline(), execution)
                        .await
                        .expect("could not handle execute receives");
                }
            }

            self.inner
                .close()
                .await
                .expect("could not close ssh channel");
        });
    }
}

#[derive(Debug, Clone)]
pub struct SSHExecution {
    pub command: String,
    pub timeout: Option<Duration>,
    pub redirected_output: bool,
    pub hold_until_end: bool,
}

impl SSHExecution {
    pub fn with_timeout(&self, timeout: Duration) -> SSHExecution {
        Self {
            timeout: Some(timeout),
            ..self.clone()
        }
    }

    pub fn without_timeout(&self) -> SSHExecution {
        Self {
            timeout: None,
            ..self.clone()
        }
    }

    pub fn with_redirected_output(&self) -> SSHExecution {
        Self {
            redirected_output: true,
            ..self.clone()
        }
    }

    pub fn without_redirected_output(&self) -> SSHExecution {
        Self {
            redirected_output: false,
            ..self.clone()
        }
    }

    pub fn with_hold_until_end(&self) -> SSHExecution {
        Self {
            hold_until_end: true,
            ..self.clone()
        }
    }

    pub fn without_hold_until_end(&self) -> SSHExecution {
        Self {
            hold_until_end: false,
            ..self.clone()
        }
    }

    pub fn deadline(&self) -> Option<Instant> {
        match self.timeout {
            None => None,
            Some(timeout) => Some(Instant::now().add(timeout)),
        }
    }
}

impl Default for SSHExecution {
    fn default() -> Self {
        Self {
            command: String::new(),
            timeout: None,
            redirected_output: false,
            hold_until_end: true,
        }
    }
}

impl From<String> for SSHExecution {
    fn from(value: String) -> Self {
        Self {
            command: value,
            ..Default::default()
        }
    }
}

impl From<&str> for SSHExecution {
    fn from(value: &str) -> Self {
        Self::from(value.to_string())
    }
}

#[derive(Debug, Clone)]
pub struct SSHExecutionResult {
    pub data: String,
    pub exit: Option<i32>,
}

impl SSHChannel {
    async fn handle_execute_receives(
        &mut self,
        deadline: Option<Instant>,
        execution: SSHExecution,
    ) -> Result<SSHExecutionResult, SSHError> {
        let mut buffer = String::new();
        let mut exit: Option<i32> = None;

        loop {
            let Some(data) = (match deadline {
                None => self.inner.wait().await,
                Some(deadline) => tokio::time::timeout_at(deadline, self.inner.wait()).await?,
            }) else {
                continue;
            };

            match data {
                ChannelMsg::Data { data } => {
                    match str::from_utf8(data.to_vec().as_slice()) {
                        Ok(v) => {
                            if execution.redirected_output {
                                info!("remote/ssh: {v}", v = v.trim_end_matches("\n"))
                            }
                            buffer.push_str(v)
                        }
                        Err(e) => return Err(SSHError::Parse(e.to_string())),
                    };
                }
                ChannelMsg::ExitStatus { exit_status } => {
                    exit = Some(exit_status as i32);
                }
                ChannelMsg::Eof => {
                    return if let Some(exit) = exit {
                        Ok(SSHExecutionResult {
                            data: buffer,
                            exit: Some(exit),
                        })
                    } else {
                        Ok(SSHExecutionResult {
                            data: buffer,
                            exit: None,
                        })
                    };
                }
                _ => continue,
            }
        }
    }

    pub async fn execute(
        &mut self,
        execution: impl Into<SSHExecution>,
    ) -> Result<SSHExecutionResult, SSHError> {
        let execution = execution.into();
        let deadline = execution.deadline();

        self.execution = Some(execution.clone());

        if let Err(e) = match deadline {
            None => self.inner.exec(true, execution.clone().command).await,
            Some(deadline) => {
                tokio::time::timeout_at(deadline, self.inner.exec(true, execution.clone().command))
                    .await
                    .map_err(|e| {
                        self.execution = None;
                        return e;
                    })?
            }
        } {
            self.execution = None;
            return Err(e.into());
        }

        let result = self
            .handle_execute_receives(deadline, execution)
            .await
            .map_err(|e| {
                self.execution = None;
                e
            });

        self.execution = None;
        result
    }

    /// Resolve uid of current user by executing 'id' command
    pub async fn resolve_uid(&mut self) -> Result<i32, SSHError> {
        let result = self.execute("id -u").await?;
        match result.data.parse::<i32>() {
            Ok(v) => Ok(v),
            Err(e) => Err(SSHError::Parse(e.to_string())),
        }
    }

    /// Resolve home directory of current user by executing 'getent' command
    pub async fn resolve_home(&mut self) -> Result<String, SSHError> {
        let uid = self.resolve_uid().await?;

        /// resolve entry
        let result = self.execute(format!("getent passwd {uid}")).await?;
        let home = result.data.split(":").nth(5);
        match home {
            None => Err(SSHError::Parse(String::from("getent_unknown_entry"))),
            Some(home) => Ok(String::from(home)),
        }
    }
}
