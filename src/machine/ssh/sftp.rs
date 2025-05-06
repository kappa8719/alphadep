use crate::machine::ssh::error::SftpError;
use russh_sftp::client::SftpSession;
use russh_sftp::client::fs::File;
use russh_sftp::protocol::{FileAttributes, OpenFlags};
use std::path::Path;

pub struct Sftp {
    pub session: SftpSession,
}

impl Sftp {
    pub fn new(session: SftpSession) -> Sftp {
        Self { session }
    }
}

impl Drop for Sftp {
    fn drop(&mut self) {
        futures::executor::block_on(self.session.close()).expect("could not close sftp session");
    }
}

#[derive(Default)]
pub struct SftpOpenOptions {
    pub flags: OpenFlags,
    pub attributes: FileAttributes,
}

impl SftpOpenOptions {
    pub fn from_flags(flags: OpenFlags) -> SftpOpenOptions {
        Self {
            flags,
            attributes: FileAttributes::default(),
        }
    }

    pub fn from_attributes(attributes: FileAttributes) -> SftpOpenOptions {
        Self {
            flags: OpenFlags::default(),
            attributes,
        }
    }

    pub fn with_flags(&self, flags: OpenFlags) -> SftpOpenOptions {
        Self {
            flags,
            attributes: self.attributes.clone(),
        }
    }

    pub fn with_attributes(&self, attributes: FileAttributes) -> SftpOpenOptions {
        Self {
            flags: self.flags.clone(),
            attributes,
        }
    }
}

impl Sftp {
    pub async fn create_dir<P: AsRef<Path>>(&self, path: P) -> Result<(), SftpError> {
        self.session
            .create_dir(path.as_ref().to_str().unwrap())
            .await?;

        Ok(())
    }

    pub async fn create_dir_all<P: AsRef<Path>>(&self, path: P) -> Result<(), SftpError> {
        let mut paths = vec![path.as_ref()];
        while let Some(parent) = paths.first().map(|p| p.parent()).flatten() {
            paths.insert(0, parent);
        }

        for path in paths {
            let str = path.to_str().unwrap();

            let exists = self.session.try_exists(str).await?;
            if exists {
                let metadata = self.session.metadata(str).await?;
                if metadata.is_dir() {
                    continue;
                }
            }

            self.session.create_dir(str).await?;
        }

        Ok(())
    }

    pub async fn open<P: AsRef<Path>>(&self, path: P) -> Result<File, SftpError> {
        let path = path.as_ref().to_str().unwrap();
        Ok(self.session.open(path).await?)
    }

    pub async fn open_with_options<P: AsRef<Path>>(
        &self,
        path: P,
        options: SftpOpenOptions,
    ) -> Result<File, SftpError> {
        let path = path.as_ref().to_str().unwrap();
        Ok(self
            .session
            .open_with_flags_and_attributes(path, options.flags, options.attributes)
            .await?)
    }

    pub async fn close(&self) -> Result<(), SftpError> {
        Ok(self.session.close().await?)
    }
}
