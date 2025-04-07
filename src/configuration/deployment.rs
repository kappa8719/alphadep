use glob::{GlobError, GlobResult, Paths, PatternError, glob};
use russh::keys::signature::digest::typenum::op;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io;
use std::io::{Seek, Write};
use std::iter::zip;
use std::path::{Path, PathBuf};
use thiserror::Error;
use zip::ZipWriter;
use zip::result::ZipError;
use zip::write::SimpleFileOptions;

#[derive(Deserialize, Debug, Default, Clone)]
pub enum DeploymentRuntimeContext {
    #[serde(rename = "session")]
    #[default]
    Session,
    #[serde(rename = "service")]
    Service,
}

#[derive(Deserialize, Debug, Default, Clone)]
pub struct DeploymentRuntime {
    #[serde(default)]
    pub context: DeploymentRuntimeContext,
    pub execute: String,
}

#[derive(Deserialize, Debug, Default, Clone)]
pub struct DeploymentFiles {
    #[serde(default)]
    pub excludes: Vec<String>,
    #[serde(default)]
    pub includes: Vec<String>,
}

#[derive(Error, Debug)]
pub enum DeploymentFileGlobError {
    PatternError(#[from] glob::PatternError),
    GlobError { path: PathBuf, error: io::ErrorKind },
}

#[derive(Error, Debug)]
pub enum DeploymentFileArchiveError {
    ZipError(#[from] ZipError),
    GlobError(#[from] DeploymentFileGlobError),
    CopyError(#[from] io::Error),
}

impl Display for DeploymentFileGlobError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl Display for DeploymentFileArchiveError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl DeploymentFiles {
    fn default_excludes() -> Vec<String> {
        vec![
            "./alphadep.toml".to_string(),
            "./.git".to_string(),
            "./.env".to_string(),
        ]
    }

    fn glob_all(vec: Vec<String>) -> Result<Vec<PathBuf>, DeploymentFileGlobError> {
        Ok(vec
            .iter()
            .map(|p| glob(p.as_str()).map(|p| p.collect::<Vec<_>>()))
            .collect::<Result<Vec<_>, _>>()?
            .iter()
            .flatten()
            .map(|v| match v {
                Ok(ok) => Ok(ok.clone()),
                Err(err) => Err(DeploymentFileGlobError::GlobError {
                    path: err.path().to_path_buf(),
                    error: err.error().kind(),
                }),
            })
            .collect::<Result<Vec<_>, _>>()?)
    }

    /// Get parsed glob paths from exclude list as Result<Vec<Paths>, PatternError>
    pub fn excludes(&self) -> Result<Vec<PathBuf>, DeploymentFileGlobError> {
        Self::glob_all([self.excludes.clone(), Self::default_excludes()].concat())
    }

    /// Get parsed glob paths from include list as Result<Vec<Paths>, PatternError>
    pub fn includes(&self) -> Result<Vec<PathBuf>, DeploymentFileGlobError> {
        Self::glob_all(self.includes.clone())
    }

    pub fn list(&self) -> Result<Vec<PathBuf>, DeploymentFileGlobError> {
        let mut all = glob("**/*")?.collect::<Result<Vec<_>, _>>().map_err(|e| {
            DeploymentFileGlobError::GlobError {
                path: e.path().to_path_buf(),
                error: e.error().kind(),
            }
        })?;
        let mut excludes = self.excludes()?;
        let includes = self.includes()?;

        excludes.retain(|v| !includes.contains(v));
        all.retain(|v| !excludes.contains(v));

        Ok(all)
    }

    pub fn write_archive<T: Write + Seek, P: AsRef<Path>>(
        &self,
        writer: T,
        excludes: Vec<P>,
    ) -> Result<(), DeploymentFileArchiveError> {
        let mut zip = ZipWriter::new(writer);
        let options = SimpleFileOptions::default();

        let mut list = self.list()?;
        excludes
            .iter()
            .map(|p| p.as_ref().canonicalize())
            .filter_map(|p| p.ok())
            .for_each(|exclude| {
                list.retain(|p| match p.clone().canonicalize() {
                    Ok(p) => p != exclude,
                    Err(_) => true,
                })
            });

        let list = list;

        for path in list {
            if path.is_dir() {
                zip.add_directory_from_path(path, options)?;
            } else {
                zip.start_file_from_path(path.clone(), options)?;
                let mut source = File::open(path)?;
                std::io::copy(&mut source, &mut zip)?;
            }
        }

        zip.finish()?;

        Ok(())
    }
}

#[derive(Deserialize, Debug, Default, Clone)]
pub enum DeploymentBuildMachine {
    #[serde(rename = "master")]
    #[default]
    Master,
    #[serde(rename = "target")]
    Target,
}

#[derive(Deserialize, Debug, Default, Clone)]
pub struct DeploymentBuild {
    #[serde(default)]
    pub machine: DeploymentBuildMachine,
    #[serde(default)]
    pub script: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct DeploymentConfiguration {
    pub id: String,
    pub runtime: DeploymentRuntime,
    #[serde(default)]
    pub files: DeploymentFiles,
    #[serde(default)]
    pub build: DeploymentBuild,
    #[serde(rename = "environment-variables", default)]
    pub environment_variables: HashMap<String, String>,
}
