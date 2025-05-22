use ssh2::ErrorCode;
use std::path::Path;

pub trait SftpExt {
    fn mkdir_recursive(&self, filename: &Path, mode: i32) -> Result<(), ssh2::Error>;
}

impl SftpExt for ssh2::Sftp {
    fn mkdir_recursive(&self, filename: &Path, mode: i32) -> Result<(), ssh2::Error> {
        let path = filename;
        let mut ancestors = path.ancestors().collect::<Vec<_>>();
        ancestors.reverse();
        let ancestors = ancestors;

        for ancestor in ancestors {
            match self.stat(ancestor) {
                Ok(_) => continue,
                Err(error) => match error.code() {
                    ErrorCode::Session(_) => return Err(error),
                    ErrorCode::SFTP(code) => {
                        if code != libssh2_sys::LIBSSH2_FX_NO_SUCH_FILE {
                            return Err(error);
                        }
                    }
                },
            }

            self.mkdir(ancestor, mode)?
        }

        Ok(())
    }
}
