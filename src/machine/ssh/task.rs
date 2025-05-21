use crate::machine::ssh::error::{ArchiveExtractError, ArchiveUploadError, SftpError};
use interface::DeploymentFiles;
use std::io::Write;
use std::path::{Path, PathBuf};

pub struct UploadRuntime<'t, P: AsRef<Path>> {
    pub sftp: ssh2::Sftp,
    pub binary: &'t [u8],
    pub destination: P,
}

impl<P: AsRef<Path>> UploadRuntime<'_, P> {
    pub fn upload(&self) -> Result<(), SftpError> {
        let destination = self.destination.as_ref();
        if let Some(parent) = destination.parent() {
            self.sftp.mkdir(parent, 0o700)?;
        }

        let mut file = self.sftp.open(self.destination.as_ref())?;
        file.write_all(self.binary)?;

        Ok(())
    }
}

pub struct UploadArchive<'t, P: AsRef<Path>> {
    pub sftp: ssh2::Sftp,
    pub files: &'t DeploymentFiles,
    pub destination: P,
}

impl<P: AsRef<Path>> UploadArchive<'_, P> {
    pub fn upload(&self) -> Result<(), ArchiveUploadError> {
        let destination = self.destination.as_ref();
        if let Some(parent) = destination.parent() {
            self.sftp.mkdir(parent, 0o700)?;
        }

        let remote_archive = self.sftp.open(self.destination.as_ref())?;
        self.files
            .write_archive(remote_archive, Vec::<PathBuf>::new())?;

        Ok(())
    }
}

pub struct ExtractArchive<P: AsRef<Path>> {
    pub channel: ssh2::Channel,
    /// the archive file at remote
    pub archive: P,
    /// the directory archive should be extracted
    pub destination: P,
    /// the path of the runtime executable
    pub runtime: P,
}

impl<P: AsRef<Path>> ExtractArchive<P> {
    pub fn extract(&mut self) -> Result<(), ArchiveExtractError> {
        let runtime = self.runtime.as_ref().to_str().unwrap();
        let archive = self.archive.as_ref().to_str().unwrap();
        let destination = self.destination.as_ref().to_str().unwrap();

        self.channel.exec(
            format!("{runtime} extract --archive {archive} --destination {destination}").as_str(),
        )?;

        loop {
            self.channel.wait_eof()?;

            if self.channel.eof() {
                let Ok(status) = self.channel.exit_status() else {
                    return Err(ArchiveExtractError::Unknown);
                };

                if status != 0 {
                    return Err(ArchiveExtractError::Exited(status));
                }

                return Ok(());
            }
        }
    }
}
