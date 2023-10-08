use std::fs::{File, read, remove_file};
use std::io::Write;
use std::path::Path;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use walkdir::WalkDir;
use crate::fs_utils::file_path_relative_to;

pub mod flame;
pub mod ftb;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PackManifest {
    pub files: Vec<String>,
}

#[derive(Debug)]
pub struct PackManifestBuilder {
    files: Vec<String>,
}

impl PackManifest {
    pub fn builder() -> PackManifestBuilder {
        PackManifestBuilder::new()
    }

    pub fn save_to<T: AsRef<Path>>(
        &self,
        file_path: T,
    ) -> anyhow::Result<()> {
        if file_path.as_ref().is_file() {
            remove_file(&file_path)?;
        }

        let bytes = serde_json::to_vec_pretty(&self)?;
        let mut file = File::create(file_path)?;
        file.write_all(&bytes)?;

        Ok(())
    }

    pub fn load_from<T: AsRef<Path>>(
        file_path: T,
    ) -> anyhow::Result<Self> {
        if !file_path.as_ref().is_file() {
            return Err(ManifestError::ManifestNotFound)?;
        }

        let bytes = read(file_path)?;
        let manifest = serde_json::from_slice(&bytes)?;

        Ok(manifest)
    }
}

impl PackManifestBuilder {
    fn new() -> Self {
        PackManifestBuilder {
            files: Vec::new(),
        }
    }

    pub fn finish(self) -> PackManifest {
        PackManifest {
            files: self.files,
        }
    }

    pub fn with_files_from_dir<T: AsRef<Path>>(
        mut self,
        dir: T,
    ) -> Self {
        for entry in WalkDir::new(&dir) {
            let entry = entry.unwrap();
            if entry.path().is_dir() {
                continue;
            }

            let relative = file_path_relative_to(&entry.path(), &dir)
                .unwrap();

            self.files.push(String::from(relative.to_str().unwrap()));
        }

        self
    }

    pub fn exclude_files_from_dir<T: AsRef<Path>>(
        mut self,
        dir: T,
    ) -> Self {
        let dir = dir.as_ref();
        if !dir.is_dir() {
            return self;
        }

        let dir = dir.to_str().unwrap().to_owned();
        self.files.retain(|entry| entry.contains(&dir));

        self
    }
}

#[derive(Error, Debug)]
pub enum ManifestError {
    #[error("The manifest was not found!")]
    ManifestNotFound,
}