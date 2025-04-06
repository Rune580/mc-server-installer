use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use async_process::Command;
use futures_util::{AsyncBufReadExt, io, StreamExt};
use futures_util::io::BufReader;
use indicatif::ProgressBar;
use log::{error, info};
use thiserror::Error;
use tokio::fs::{remove_dir_all, remove_file};
use crate::cli;
use crate::fs_utils::{download_file, work_dir};
use crate::modpack::ftb::client::FtbClient;
use crate::modpack::{check_manifest, post_process};

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
) -> color_eyre::Result<()> {
    let mut ctx = Context {
        client: FtbClient::new(),
        args,
        version,
        pack_id: None,
        version_id: None,
        installer_path: None,
        target_dir: target_dir.as_ref().to_path_buf(),
    };

    check_manifest(&ctx.target_dir).await?;
    setup()?;
    resolve_pack_id(&mut ctx).await?;
    resolve_version_id(&mut ctx).await?;
    download_server_installer(&mut ctx).await?;
    #[cfg(target_os = "linux")]
    linux_make_installer_executable(&mut ctx).await?;
    install_server(&mut ctx).await?;
    post_process(&ctx.target_dir).await?;

    Ok(())
}



fn setup() -> color_eyre::Result<()> {
    let dir = PathBuf::from(".mcsi");
    if !dir.exists() {
        std::fs::create_dir(dir)?;
    }

    Ok(())
}

async fn resolve_pack_id(ctx: &mut Context) -> color_eyre::Result<()> {
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

async fn resolve_version_id(ctx: &mut Context) -> color_eyre::Result<()> {
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

async fn download_server_installer(ctx: &mut Context) -> color_eyre::Result<()> {
    let pack_id = ctx.pack_id.unwrap();
    let version_id = ctx.version_id.unwrap();

    let url = format!("https://api.modpacks.ch/public/modpack/{pack_id}/{version_id}/server/{TARGET_OS}");
    let dst = PathBuf::from("./.mcsi")
        .join(installer_file_name(pack_id, version_id));

    info!("downloading installer...");
    let path = download_file(&url, &dst)
        .await?;

    ctx.installer_path = Some(path.to_str().unwrap().to_string());

    Ok(())
}

#[cfg(target_os = "linux")]
async fn linux_make_installer_executable(ctx: &mut Context) -> color_eyre::Result<()> {
    let installer = ctx.installer_path.clone().unwrap();

    Command::new("chmod")
        .args(["+x", &installer])
        .output()
        .await?;

    Ok(())
}

async fn install_server(ctx: &mut Context) -> color_eyre::Result<()> {
    let work_dir = work_dir();
    if work_dir.is_dir() {
        remove_dir_all(&work_dir)
            .await?;
    }

    let installer = ctx.installer_path.clone().unwrap();

    let install_progress = ProgressBar::new_spinner()
        .with_style(cli::spinner_progress_style())
        .with_prefix("Running FTB installer");

    install_progress.enable_steady_tick(Duration::from_millis(500));

    let mut child = Command::new(installer.clone())
        .args(["--auto", "--path", "./.mcsi/work_dir", "--nojava"])
        .stdout(Stdio::piped())
        .spawn()?;

    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    while let Some(line) = lines.next().await {
        if let Ok(line) = line {
            install_progress.set_message(line);
        }
    }

    let wait = async move {
        child.status().await?;
        io::Result::Ok(false)
    };
    wait.await?;

    install_progress.finish();
    remove_file(installer)
        .await?;

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