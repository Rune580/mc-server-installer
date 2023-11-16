use std::fs::{File, read, remove_file};
use std::io::Write;
use std::path::{Path, PathBuf};
use chrono::Utc;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::fs::{create_dir_all, remove_dir_all};
use walkdir::WalkDir;
use crate::fs_utils::{backup_and_remove_files, file_path_relative_to, logs_dir, mcsi_dir, recursive_copy_to_dir, work_dir};

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
        mcsi_dir: T,
    ) -> anyhow::Result<()> {
        let file_path = mcsi_dir.as_ref()
            .join("manifest.json");
        if file_path.is_file() {
            remove_file(&file_path)?;
        }

        let bytes = serde_json::to_vec_pretty(&self)?;
        let mut file = File::create(file_path)?;
        file.write_all(&bytes)?;

        Ok(())
    }

    pub fn load_from<T: AsRef<Path>>(
        mcsi_dir: T,
    ) -> anyhow::Result<Self> {
        let file_path = mcsi_dir.as_ref()
            .join("manifest.json");
        if !file_path.is_file() {
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
        let mut files = 0;
        for entry in WalkDir::new(&dir) {
            let entry = entry.unwrap();
            if entry.path().is_dir() {
                continue;
            }

            let relative = file_path_relative_to(entry.path(), &dir)
                .unwrap();

            self.files.push(String::from(relative.to_str().unwrap()));
            files += 1;
        }
        debug!("Added {} files to manifest. {} Total files", files, self.files.len());

        self
    }

    pub fn exclude_files_from_dir<T: AsRef<Path>>(
        mut self,
        dir: T,
    ) -> Self {
        let dir = PathBuf::from(dir.as_ref());
        // if !dir.is_dir() {
        //     return self;
        // }

        let prev = self.files.len();

        let dir = dir.to_str().unwrap().to_owned();
        debug!("Excluding files from {}", dir);

        self.files.retain(|entry| !entry.contains(&dir));

        debug!("Excluded {} files from manifest", prev - self.files.len());

        self
    }
}

#[derive(Error, Debug)]
pub enum ManifestError {
    #[error("The manifest was not found!")]
    ManifestNotFound,
}

pub async fn check_manifest<T: AsRef<Path>>(target_dir: T) -> anyhow::Result<()> {
    let target_dir = target_dir.as_ref();
    let mcsi_dir = target_dir
        .join(".mcsi");

    let manifest_path = mcsi_dir
        .join("manifest.json");

    if !manifest_path.is_file() {
        return Ok(());
    }

    let manifest = PackManifest::load_from(&mcsi_dir)?;
    info!("Existing pack manifest found!");

    let now = Utc::now().format("%Y-%m-%d-%H%M%S").to_string();
    let backup_dir = mcsi_dir
        .join("backups")
        .join(format!("backup-{now}"));

    create_dir_all(&backup_dir)
        .await?;

    backup_and_remove_files(target_dir, backup_dir, manifest.files)
        .await?;

    Ok(())
}

pub async fn post_process<T: AsRef<Path>>(target_dir: T) -> anyhow::Result<()> {
    info!("Finishing up...");

    let target_dir = target_dir.as_ref();
    let work_dir = work_dir();

    recursive_copy_to_dir(&work_dir, &target_dir)
        .await?;

    remove_dir_all(&work_dir)
        .await?;

    let target_mcsi_dir = target_dir
        .join(".mcsi");
    if !target_mcsi_dir.is_dir() {
        create_dir_all(&target_mcsi_dir)
            .await?;
    }

    let target_logs_dir = target_mcsi_dir
        .join("logs");
    if !target_logs_dir.is_dir() {
        create_dir_all(&target_logs_dir)
            .await?;
    }

    #[cfg(target_os = "linux")]
    let pack_manifest = PackManifest::builder()
        .with_files_from_dir(target_dir)
        .exclude_files_from_dir(".mcsi/")
        .finish();

    #[cfg(target_os = "windows")]
        let pack_manifest = PackManifest::builder()
        .with_files_from_dir(target_dir)
        .exclude_files_from_dir(".mcsi\\")
        .finish();

    pack_manifest.save_to(&target_mcsi_dir)?;

    recursive_copy_to_dir(logs_dir(), target_logs_dir)
        .await?;

    let src_mcsi_dir = mcsi_dir();
    if !src_mcsi_dir.eq(&target_mcsi_dir) {
        remove_dir_all(src_mcsi_dir)
            .await?;
    }

    println!("Server is installed!");
    Ok(())
}