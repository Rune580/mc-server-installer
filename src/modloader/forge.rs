use std::fs::remove_file;
use std::path::Path;
use std::process::Command;
use log::{error, info};
use crate::fs_utils::download_file;
use crate::version::McVersion;

pub async fn install_forge<P: AsRef<Path>>(mc_version: McVersion, forge_version: &str, work_dir: P) -> anyhow::Result<()> {
    let long_version = format!("{mc_version}-{forge_version}", mc_version = mc_version.as_str());
    let installer = format!("forge-{long_version}-installer.jar");
    let universal = format!("forge-{long_version}-universal.jar");

    let installer_url = format!("https://maven.minecraftforge.net/net/minecraftforge/forge/{long_version}/{installer}");
    let universal_url = format!("https://maven.minecraftforge.net/net/minecraftforge/forge/{long_version}/{universal}");

    let installer_dst = work_dir.as_ref().join("installer.jar");
    let universal_dst = work_dir.as_ref().join("server.jar");

    println!("Downloading forge...");
    download_file(&installer_url, &installer_dst)
        .await?;
    download_file(&universal_url, &universal_dst)
        .await?;

    println!("Installing forge...");
    let output = Command::new("java")
        .current_dir(&work_dir)
        .args(["-jar", "installer.jar", "--installServer"])
        .output()?;

    if !output.status.success() {
        error!("Forge install failed\nCode:\t{:#?}\nOutput:\n{:#?}", output.status.code(), output)

    } else {
        info!("Forge installed successfully!")
    }

    remove_file(installer_dst)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs::create_dir;
    use std::path::PathBuf;
    use std::str::FromStr;
    use crate::modloader::forge::install_forge;
    use crate::version::McVersion;

    #[tokio::test]
    async fn download_forge() {
        let mc_version = McVersion::from_str("1.19.2").unwrap();
        let forge_version = "43.2.21";

        let work_dir = PathBuf::new().join("tmp");
        if !work_dir.exists() {
            create_dir(&work_dir).unwrap();
        }

        install_forge(mc_version, forge_version, &work_dir)
            .await
            .unwrap();
    }
}