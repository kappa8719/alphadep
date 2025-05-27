use crate::machine::ssh::error::{
    ArchiveExtractError, ArchiveUploadError, ManifestUploadError, SftpError,
};
use crate::machine::ssh::sftp::SftpExt;
use crate::machine::ssh::ssh::ChannelExt;
use interface::{DeploymentFiles, RuntimeManifest};
use ssh2::{OpenFlags, OpenType};
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
            self.sftp.mkdir_recursive(parent, 0o700)?;
        }

        let mut file = self.sftp.open_mode(
            destination,
            OpenFlags::WRITE | OpenFlags::TRUNCATE,
            0o700,
            OpenType::File,
        )?;
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
            self.sftp.mkdir_recursive(parent, 0o700)?;
        }

        let remote_archive = self.sftp.create(destination)?;
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

        self.channel.consume();

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

pub struct UploadManifest<P: AsRef<Path>> {
    pub sftp: ssh2::Sftp,
    pub destination: P,
    pub manifest: RuntimeManifest,
}

impl<P: AsRef<Path>> UploadManifest<P> {
    pub fn upload(&mut self) -> Result<(), ManifestUploadError> {
        let manifest_as_string = toml::to_string(&self.manifest)?;
        let mut file = self.sftp.create(self.destination.as_ref())?;
        file.write(manifest_as_string.as_bytes())?;

        Ok(())
    }
}
