use std::path::Path;
use crate::fs_utils::download_file;
use crate::version::McVersion;

pub async fn install_fabric<P: AsRef<Path>>(mc_version: McVersion, loader_version: &str, work_dir: P) -> color_eyre::Result<()> {
    let installer_version = latest_fabric_installer_version()
        .await?;

    let url = format!("https://meta.fabricmc.net/v2/versions/loader/{0}/{1}/{2}/server/jar", mc_version.as_str(), loader_version, installer_version);
    let dst = work_dir.as_ref().join("server.jar");

    download_file(&url, dst)
        .await?;

    Ok(())
}

async fn latest_fabric_installer_version() -> color_eyre::Result<String> {
    let resp = reqwest::get("https://meta.fabricmc.net/v2/versions/installer")
        .await?
        .text()
        .await?;

    let json: serde_json::Value = serde_json::from_str(&resp)?;
    let version = json.get(0).unwrap().get("version").unwrap().as_str().unwrap();

    Ok(version.to_string())
}