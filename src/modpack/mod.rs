use std::fs::{File, read, remove_file};
use std::io::Write;
use std::path::{Path, PathBuf};
use chrono::Utc;
use futures_util::AsyncWriteExt;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::fs::{create_dir_all, remove_dir_all};
use walkdir::WalkDir;
use crate::fs_utils::{backup_and_remove_files, file_path_relative_to, get_server_start_script, logs_dir, mcsi_dir, recursive_copy_to_dir, set_as_executable, work_dir};
use crate::modloader::ModLoader;

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
    ) -> color_eyre::Result<()> {
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
    ) -> color_eyre::Result<Self> {
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

pub async fn check_manifest<T: AsRef<Path>>(target_dir: T) -> color_eyre::Result<()> {
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

fn create_mc_start_script() -> color_eyre::Result<File> {
    let start_script_path = work_dir()
        .join("mc-start.sh");

    let file = File::create(&start_script_path)?;

    Ok(file)
}

#[cfg(target_os = "linux")]
fn make_mc_start_script_executable() -> color_eyre::Result<()> {
    let start_script_path = work_dir()
        .join("mc-start.sh");

    set_as_executable(start_script_path)
}

pub async fn ensure_server_start_script(mod_loader: Option<ModLoader>) -> color_eyre::Result<()> {
    let start_script = get_server_start_script(work_dir());

    if start_script.is_some() {
        let mut mc_start_file = create_mc_start_script()?;
        write!(&mut mc_start_file, "#!/usr/bin/env sh\n{0}", start_script.unwrap().to_str().unwrap())?;
        mc_start_file.flush()?;

        #[cfg(target_os = "linux")]
        make_mc_start_script_executable()?;

        return Ok(());
    }

    if let Some(mod_loader) = mod_loader {
        match mod_loader {
            ModLoader::NeoForge { .. } => {
                let mut mc_start_file = create_mc_start_script()?;
                #[cfg(target_os = "linux")]
                write!(&mut mc_start_file, "#!/usr/bin/env sh\njava -Xms128M -Xmx${{SERVER_MEMORY}}M -jar server.jar")?;

                #[cfg(target_os = "linux")]
                make_mc_start_script_executable()?;
            }
            ModLoader::Forge { .. } => {
                let mut mc_start_file = create_mc_start_script()?;
                #[cfg(target_os = "linux")]
                write!(&mut mc_start_file, "#!/usr/bin/env sh\njava -Xms128M -Xmx${{SERVER_MEMORY}}M -jar server.jar")?;

                #[cfg(target_os = "linux")]
                make_mc_start_script_executable()?;
            }
            ModLoader::Fabric { .. } => {
                todo!()
            }
            ModLoader::Quilt { .. } => {
                todo!()
            }
        }
    }

    Ok(())
}

pub async fn post_process<T: AsRef<Path>>(target_dir: T) -> color_eyre::Result<()> {
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

    info!("Server is installed!");
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use log::LevelFilter;
    use simplelog::{ColorChoice, CombinedLogger, TermLogger, TerminalMode, WriteLogger};
    use crate::fs_utils::get_log_file;
    use crate::modpack::flame;

    fn init_logging() {
        CombinedLogger::init(
            vec![
                TermLogger::new(LevelFilter::Debug, simplelog::Config::default(), TerminalMode::Mixed, ColorChoice::Auto),
                WriteLogger::new(LevelFilter::Debug, simplelog::Config::default(), get_log_file().unwrap()),
            ]
        ).unwrap();
    }

    async fn test_flame_pack(project_id: u64, version: &str) -> color_eyre::Result<()> {
        init_logging();
        dotenvy::dotenv().ok();
        let api_key = std::env::var("API_KEY")?;

        let target_dir = PathBuf::from("./.mcsi-test-dir")
            .join("tests")
            .join(format!("flame-{project_id}-{version}"));

        flame::handle_flame(api_key, project_id, version.to_string(), target_dir)
            .await?;

        Ok(())
    }

    #[tokio::test]
    async fn download_flame_pack_with_no_server_pack_1() {
        let project_id = 351508;
        let version = "6822909";

        test_flame_pack(project_id, version)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn download_flame_pack_with_server_pack_1() {
        let project_id = 925200;
        let version = "6826503";

        test_flame_pack(project_id, version)
            .await
            .unwrap();
    }
}