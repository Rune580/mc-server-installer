use std::path::{Path, PathBuf};
use std::process::Command;
use log::{error, info};
use thiserror::Error;
use tokio::fs::{create_dir_all, remove_dir_all};
use crate::fs_utils::{download_file, work_dir};
use crate::modpack::ftb::client::FtbClient;

mod model;
mod client;

#[derive(Clone, Debug)]
pub enum IdOrSearch {
    Id(String),
    Search {
        terms: Vec<String>,
        mc_version: Option<String>,
    }
}

#[derive(Clone, Debug)]
struct Context {
    client: FtbClient,
    args: IdOrSearch,
    version: String,
    pack_id: Option<usize>,
    version_id: Option<usize>,
    installer_path: Option<String>,
    target_dir: PathBuf,
}

pub async fn handle_ftb<T: AsRef<Path>>(
    args: IdOrSearch,
    version: String,
    target_dir: T,
) -> anyhow::Result<()> {
    let mut ctx = Context {
        client: FtbClient::new(),
        args,
        version,
        pack_id: None,
        version_id: None,
        installer_path: None,
        target_dir: target_dir.as_ref().to_path_buf(),
    };

    setup()?;
    resolve_pack_id(&mut ctx).await?;
    resolve_version_id(&mut ctx).await?;
    download_server_installer(&mut ctx).await?;
    install_server(&mut ctx).await?;

    Ok(())
}

fn setup() -> anyhow::Result<()> {
    let dir = PathBuf::from(".mcsi");
    if !dir.exists() {
        std::fs::create_dir(dir)?;
    }

    Ok(())
}

async fn resolve_pack_id(ctx: &mut Context) -> anyhow::Result<()> {
    let id = match &ctx.args {
        IdOrSearch::Id(id) => id.parse::<usize>().unwrap(),
        IdOrSearch::Search {
            terms,
            mc_version: _
        } => {
            info!("Searching for best matching pack...");
            let results = ctx.client.search(terms)
                .await?;

            results.packs.first().unwrap().clone()
        }
    };

    ctx.pack_id = Some(id);

    Ok(())
}

async fn resolve_version_id(ctx: &mut Context) -> anyhow::Result<()> {
    let pack_id = ctx.pack_id.unwrap();
    let mut details = ctx.client.get_pack_details(pack_id)
        .await?;

    if ctx.version.eq_ignore_ascii_case("latest") {
        details.versions.sort_by(|a, b| b.updated.cmp(&a.updated));

        let latest = details.versions.first().unwrap();

        ctx.version_id = Some(latest.id);
    } else {
        let preferred_version_id: usize = ctx.version.parse()?;

        let version_valid = details.versions.iter().any(|entry| entry.id == preferred_version_id);
        if !version_valid {
            return Err(FtbError::InvalidVersion)?;
        }

        ctx.version_id = Some(preferred_version_id);
    }

    Ok(())
}

async fn download_server_installer(ctx: &mut Context) -> anyhow::Result<()> {
    let work_dir = work_dir();
    if !work_dir.is_dir() {
        create_dir_all(&work_dir)
            .await?;
    }

    let pack_id = ctx.pack_id.unwrap();
    let version_id = ctx.version_id.unwrap();

    let url = format!("https://api.modpacks.ch/public/modpack/{pack_id}/{version_id}/server/{TARGET_OS}");
    let dst = work_dir
        .join(installer_file_name(pack_id, version_id));

    info!("downloading installer...");
    let path = download_file(&url, &dst)
        .await?;

    ctx.installer_path = Some(path.to_str().unwrap().to_string());

    Ok(())
}

async fn install_server(ctx: &mut Context) -> anyhow::Result<()> {
    let work_dir = work_dir();
    let server_dir = PathBuf::from(".mcsi")
        .join("server");

    if server_dir.is_dir() {
        remove_dir_all(&server_dir)
            .await?;
    }

    let installer = ctx.installer_path.clone().unwrap();

    info!("Installing server, this may take a few minutes...");
    let output = Command::new(installer)
        .current_dir(&work_dir)
        .args(["--auto", "--path", "../server", "--nojava"])
        .output()?;

    if !output.status.success() {
        error!("Failed to install ftb server! \nCode:\t{:#?}\nOutput:\n{:#?}", output.status.code(), output);
        return Err(FtbError::InstallerError)?;
    }

    info!("Ftb installer finished!");

    Ok(())
}

#[cfg(target_os = "windows")]
const TARGET_OS: &'static str = "windows";
#[cfg(target_os = "linux")]
const TARGET_OS: &'static str = "linux";

#[cfg(target_os = "windows")]
fn installer_file_name(
    pack_id: usize,
    version_id: usize,
) -> String {
    format!("serverinstall_{pack_id}_{version_id}.exe")
}

#[cfg(target_os = "linux")]
fn installer_file_name(
    pack_id: usize,
    version_id: usize,
) -> String {
    format!("serverinstall_{pack_id}_{version_id}")
}

#[derive(Error, Debug)]
pub enum FtbError {
    #[error("Invalid version!")]
    InvalidVersion,
    #[error("Ftb installer error")]
    InstallerError,
}