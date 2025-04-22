use russh_sftp::client::SftpSession;
use russh_sftp::client::error::Error;
use russh_sftp::protocol::Status;
use std::path::Path;
use std::{fs, io};

pub struct Sftp {
    session: SftpSession,
}

pub enum SftpError {
    /// Contains an error status packet
    #[error("{}: {}", .0.status_code, .0.error_message)]
    Status(Status),
    /// Any errors related to I/O
    #[error("I/O: {0}")]
    IO(String),
    /// Time limit for receiving response packet exceeded
    #[error("Timeout")]
    Timeout,
    /// Occurs due to exceeding the limits set by the `limits@openssh.com` extension
    #[error("Limit exceeded: {0}")]
    Limited(String),
    /// Occurs when an unexpected packet is sent
    #[error("Unexpected packet")]
    UnexpectedPacket,
    /// Occurs when unexpected server behavior differs from the protocol specification
    #[error("{0}")]
    UnexpectedBehavior(String),
}

impl From<russh_sftp::client::error::Error> for SftpError {
    fn from(value: Error) -> Self {
        match value {
            Error::Status(status) => Self::Status(status),
            Error::IO(v) => Self::IO(v),
            Error::Timeout => Self::Timeout,
            Error::Limited(v) => Self::Limited(v),
            Error::UnexpectedPacket => Self::UnexpectedPacket,
            Error::UnexpectedBehavior(v) => Self::UnexpectedBehavior(v),
        }
    }
}

impl Sftp {
    pub async fn create_dir<P: AsRef<Path>>(&self, path: P) -> Result<(), SftpError> {
        self.session.create_dir(path.as_ref().to_str().unwrap())
    }

    pub async fn create_dir_all<P: AsRef<Path>>(&self, path: P) -> Result<(), SftpError> {
        let path = path.as_ref();
        if path == Path::new("") {
            return Ok(());
        }

        match self.create_dir(path) {
            Ok(()) => return Ok(()),
            Err(_) if path.is_dir() => return Ok(()),
            Err(e) => return Err(e),
            _ => {}
        }
        match path.parent() {
            Some(p) => self.create_dir_all(p)?,
            None => {
                return Err(SftpError::IO(String::from("failed to create whole tree")));
            }
        }
        match self.create_dir(path) {
            Ok(()) => Ok(()),
            Err(_) if path.is_dir() => Ok(()),
            Err(e) => Err(e),
        }
    }
}
