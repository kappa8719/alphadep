use interface::DeploymentFileArchiveError;
use ssh2::Error;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HandshakeError {
    #[error("handshake failed due to address lookup")]
    Lookup,
    #[error("handshake failed due to io error")]
    IO(#[from] std::io::Error),
    #[error("handshake failed due to network reason")]
    Socket,
    #[error("handshake failed due to key exchange failure")]
    KeyExchange,
    #[error("unknown error occurred")]
    Unknown,
}

impl From<ssh2::Error> for HandshakeError {
    fn from(value: Error) -> Self {
        let ssh2::ErrorCode::Session(code) = value.code() else {
            return Self::Unknown;
        };

        // LIBSSH2_ERROR_SOCKET_NONE = -1 is not defined in the crate
        const LIBSSH2_ERROR_SOCKET_NONE: i32 = -1;

        match code {
            libssh2_sys::LIBSSH2_ERROR_SOCKET_SEND
            | libssh2_sys::LIBSSH2_ERROR_SOCKET_DISCONNECT
            | LIBSSH2_ERROR_SOCKET_NONE => Self::Socket,
            libssh2_sys::LIBSSH2_ERROR_KEX_FAILURE => Self::KeyExchange,
            _ => Self::Unknown,
        }
    }
}

#[derive(Error, Debug)]
pub enum AuthenticateError {
    #[error("the authentication failed by network reason")]
    Socket,
    #[error("the authentication timed out")]
    Timeout,
    #[error("the authentication failed due to key error: {0}")]
    Key(#[from] KeyError),
    #[error("the authentication failed due to invalid username/identity")]
    PublicKeyUnverified,
    #[error("the authentication failed because the supplied password has expired")]
    PasswordExpired,
    #[error("the authentication failed because the supplied key was not accepted")]
    Failed,
    #[error("the authentication failed with unknown reason")]
    Unknown,
}

impl From<ssh2::Error> for AuthenticateError {
    fn from(value: Error) -> Self {
        let ssh2::ErrorCode::Session(code) = value.code() else {
            return Self::Unknown;
        };

        match code {
            libssh2_sys::LIBSSH2_ERROR_SOCKET_SEND => Self::Socket,
            libssh2_sys::LIBSSH2_ERROR_SOCKET_TIMEOUT => Self::Timeout,
            libssh2_sys::LIBSSH2_ERROR_PUBLICKEY_UNVERIFIED => Self::PublicKeyUnverified,
            libssh2_sys::LIBSSH2_ERROR_PASSWORD_EXPIRED => Self::PasswordExpired,
            libssh2_sys::LIBSSH2_ERROR_AUTHENTICATION_FAILED => Self::Failed,
            _ => Self::Unknown,
        }
    }
}

#[derive(Error, Debug)]
pub enum KeyError {
    #[error("failed to read key due to io error")]
    IO(#[from] std::io::Error),
}

#[derive(Error, Debug)]
pub enum SftpError {
    #[error("sftp operation failed due to io error")]
    IO(#[from] std::io::Error),
    #[error("sftp operation failed for network reason")]
    Socket,
    #[error("sftp operation timed out")]
    Timeout,
    #[error("sftp operation or response was invalid")]
    Protocol,
    #[error("sftp operation failed with unknown reason")]
    Unknown,
}

impl From<ssh2::Error> for SftpError {
    fn from(value: Error) -> Self {
        let ssh2::ErrorCode::Session(code) = value.code() else {
            return Self::Unknown;
        };

        match code {
            libssh2_sys::LIBSSH2_ERROR_SOCKET_SEND => Self::Socket,
            libssh2_sys::LIBSSH2_ERROR_SOCKET_TIMEOUT => Self::Timeout,
            libssh2_sys::LIBSSH2_ERROR_SFTP_PROTOCOL => Self::Protocol,
            _ => Self::Unknown,
        }
    }
}

#[derive(Error, Debug)]
pub enum ProcessStartupError {
    #[error("process startup failed with unknown reason")]
    Unknown,
    #[error("process startup failed for network reason")]
    Socket,
    #[error("process startup request was denied")]
    Denied,
}

impl From<ssh2::Error> for ProcessStartupError {
    fn from(value: Error) -> Self {
        let ssh2::ErrorCode::Session(code) = value.code() else {
            return Self::Unknown;
        };

        match code {
            libssh2_sys::LIBSSH2_ERROR_CHANNEL_REQUEST_DENIED => Self::Denied,
            libssh2_sys::LIBSSH2_ERROR_SOCKET_SEND => Self::Socket,
            _ => Self::Unknown,
        }
    }
}

#[derive(Error, Debug)]
pub enum ArchiveUploadError {
    #[error("archive upload failed due to sftp error")]
    Sftp(#[from] SftpError),
    #[error("archive upload failed due to archiving error")]
    ArchiveError(#[from] DeploymentFileArchiveError),
}

impl From<ssh2::Error> for ArchiveUploadError {
    fn from(value: Error) -> Self {
        Self::from(SftpError::from(value))
    }
}

#[derive(Error, Debug)]
pub enum ArchiveExtractError {
    #[error("failed to start the runtime executable")]
    ProcessStartup(#[from] ProcessStartupError),
    #[error("runtime extraction exited with non-successful code {0}")]
    Exited(i32),
    #[error("runtime failed to extract with unknown reason")]
    Unknown,
}

impl From<ssh2::Error> for ArchiveExtractError {
    fn from(value: Error) -> Self {
        Self::ProcessStartup(ProcessStartupError::from(value))
    }
}
