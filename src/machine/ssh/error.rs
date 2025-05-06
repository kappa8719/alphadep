use russh::Error;
use thiserror::Error;
use tokio::time::error::Elapsed;

#[derive(Error, Debug, Clone)]
pub enum SSHError {
    #[error("protocol({0})")]
    Protocol(String),
    #[error("parse({0})")]
    Parse(String),
    #[error("disconnected")]
    Disconnected,
    #[error("timeout")]
    Timeout,
    #[error("terminated")]
    Terminated,
}

impl From<russh::Error> for SSHError {
    fn from(value: Error) -> Self {
        match value {
            Error::ConnectionTimeout | Error::InactivityTimeout | Error::KeepaliveTimeout => {
                Self::Timeout
            }
            Error::Disconnect => Self::Disconnected,
            _ => Self::Protocol(value.to_string()),
        }
    }
}

impl From<Elapsed> for SSHError {
    fn from(_: Elapsed) -> Self {
        Self::Timeout
    }
}

#[derive(Error, Debug, Clone)]
pub enum SftpError {
    /// Time limit for receiving response packet exceeded
    #[error("timeout")]
    Timeout,
    /// Any errors related to I/O
    #[error("io: {0}")]
    IO(String),
    /// Wrapper for russh_sftp client error
    #[error("protocol error: {0}")]
    Protocol(String),
}

type ProtocolError = russh_sftp::client::error::Error;

impl From<ProtocolError> for SftpError {
    fn from(value: ProtocolError) -> Self {
        match value {
            ProtocolError::Timeout => Self::Timeout,
            ProtocolError::IO(v) => Self::IO(v),
            ProtocolError::Status(status) => Self::Protocol(format!(
                "status({},{})",
                status.status_code, status.error_message
            )),
            ProtocolError::Limited(v) => Self::Protocol(format!("limited({v})")),
            ProtocolError::UnexpectedPacket => Self::Protocol(String::from("unexpected_packet")),
            ProtocolError::UnexpectedBehavior(v) => {
                Self::Protocol(format!("unexpected_behavior({v})"))
            }
        }
    }
}
