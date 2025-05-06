use std::string::FromUtf8Error;

#[derive(Clone)]
pub struct Error {
    pub source: ErrorSource,
}

#[derive(Clone)]
pub enum ErrorSource {
    Connect,
    AcquireChannel,
}

pub enum ErrorType {
    KeyRead,
    KeyUnknown,
    KeyUnsupported,
    KeyEncrypted,
    KeyCorrupted,
    KeyChanged,

    Kex,
    KexInit,

    UnknownAlgorithm,
    CommonAlgorithmNotFound,

    Version,

    PacketAuth,
    PacketSize,
    PacketFailedToDecrypt,

    Inconsistent,
    Unauthenticated,
    IndexOutOfBounds,

    InvalidServerSignature,

    ChannelNotOpened,
    ChannelOpenFail,
    ChannelSendError,

    Disconnected,
    HangUp,

    ConnectionTimeout,
    InactivityTimeout,
    KeepaliveTimeout,

    HomeDirNotFound,

    RequestDenied,
}

pub enum KeyError {
    Read,
    Unsupported,
    TypeEd25519,
    TypeEcdsa,
    Encrypted,
    Corrupted,
    Changed,
    AlgorithmUnknown,
    IndexOutOfBounds,
    Signature,
    Parameters,
    AgentProtocol,
    AgentFailure,
    MissingHomeDirectory,

    Base64Decode,
    TypeDer,
    TypeSPKI,
    TypePKCS1,
    TypePKCS8,
    TypeSec1,

    SSHKey(russh::keys::ssh_key::Error),
    SSHEncoding(russh::keys::ssh_encoding::Error),

    EnvironmentVariable(String),
    Utf8(FromUtf8Error),
    AuthSocket,
}
