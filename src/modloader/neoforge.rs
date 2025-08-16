use std::fs::remove_file;
use std::path::Path;
use std::process::Command;
use log::{error, info};
use crate::fs_utils::download_file;

pub async fn install_neoforge<P: AsRef<Path>>(neoforge_version: &str, work_dir: P) -> color_eyre::Result<()> {
    info!("Downloading neoforge...");
    
    let installer = format!("neoforge-{neoforge_version}-installer.jar");
    let universal = format!("neoforge-{neoforge_version}-universal.jar");

    let installer_url = format!("https://maven.neoforged.net/releases/net/neoforged/neoforge/{neoforge_version}/{installer}");
    let universal_url = format!("https://maven.neoforged.net/releases/net/neoforged/neoforge/{neoforge_version}/{universal}");

    let installer_dst = work_dir.as_ref().join("installer.jar");
    let universal_dst = work_dir.as_ref().join("server.jar");

    download_file(&installer_url, &installer_dst)
        .await?;
    download_file(&universal_url, &universal_dst)
        .await?;

    info!("Installing neoforge...");
    let output = Command::new("java")
        .current_dir(&work_dir)
        .args(["-jar", "installer.jar", "--installServer"])
        .output()?;

    if !output.status.success() {
        error!("NeoForge install failed\nCode:\t{:#?}\nOutput:\n{:#?}", output.status.code(), output)

    } else {
        info!("NeoForge installed successfully!")
    }

    remove_file(installer_dst)?;

    Ok(())
}