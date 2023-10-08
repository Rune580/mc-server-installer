use std::fs::File;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use log::{debug, error, info};
use reqwest::{Client, header::HeaderMap};
use thiserror::Error;
use tokio::fs::{create_dir, create_dir_all, remove_dir_all, remove_file};
use crate::fs_utils::{download_file, get_closest_common_parent, recursive_copy_to_dir, work_dir};
use crate::modloader::fabric::install_fabric;
use crate::modloader::forge::install_forge;
use crate::modloader::ModLoader;
use crate::modpack::flame::model::{ClientManifest, FileEntry, ManifestFileEntry};
use crate::modpack::PackManifest;
use crate::version::McVersion;

use self::client::FlameClient;

mod model;
mod client;
mod manifest;

#[derive(Clone, Debug)]
struct Context {
    client: FlameClient,
    project_id: u64,
    version: String,
    main_file: Option<FileEntry>,
    parent_file: Option<FileEntry>,
    mc_version: Option<McVersion>,
    mod_loader: Option<ModLoader>,
    mod_list: Option<Vec<ManifestFileEntry>>,
    target_dir: PathBuf,
}

pub async fn handle_flame<T: AsRef<Path>>(
    api_key: String,
    project_id: u64,
    version: String,
    target_dir: T,
) -> anyhow::Result<()> {
    debug!("api_key: \'{api_key}\' project_id: \'{project_id}\' version: \'{version}\'");

    let mut headers = HeaderMap::new();
    headers.insert("Accept", "application/json".parse()?);
    headers.insert("x-api-key", api_key.parse()?);

    let client = Client::builder()
        .default_headers(headers)
        .build()?;

    let mut ctx = Context {
        client: FlameClient::new(client),
        project_id,
        version,
        main_file: None,
        parent_file: None,
        mc_version: None,
        mod_loader: None,
        mod_list: None,
        target_dir: target_dir.as_ref().to_path_buf(),
    };

    setup(&mut ctx).await?;
    resolve_main_file(&mut ctx).await?;
    ensure_server_pack(&mut ctx).await?;
    download_modpack(&mut ctx).await?;
    post_process(&mut ctx).await?;

    Ok(())
}

async fn resolve_main_file(ctx: &mut Context) -> anyhow::Result<()> {
    if ctx.version.eq_ignore_ascii_case("latest") {
        info!("Version set to \'latest\', determining file id...");

        let info = ctx.client.get_mod_info(ctx.project_id)
            .await?;

        let file_id = info.main_file_id;
        let main_file = ctx.client.get_file_info(ctx.project_id, file_id)
            .await?;

        ctx.main_file = Some(main_file);
    } else {
        match u64::from_str(ctx.version.as_str()) {
            Ok(file_id) => {
                info!("Version recognized as a file id, validating id...");

                let main_file = ctx.client.get_file_info(ctx.project_id, file_id)
                    .await;

                if main_file.is_ok() {
                    ctx.main_file = Some(main_file.unwrap());
                    info!("file id is: {0}", ctx.main_file.clone().expect("file info must exist").id);
                    return Ok(())
                }
            }
            Err(_) => {}
        }
        info!("Version is not a valid file id, performing name search...");

        let mut items = 0;
        let mut page = 0;
        let mut total = u32::MAX;

        while items < total {
            let file_list = ctx.client.get_files(ctx.project_id, page)
                .await?;

            let main_file = file_list.files.iter()
                .find(|file| file.display_name.contains(ctx.version.as_str()));

            if main_file.is_some() {
                ctx.main_file = Some(main_file.expect("file must exist").clone());
                break;
            }

            total = file_list.pagination.total_count;
            items += file_list.pagination.result_count;
            page += 1;
        }
    }

    info!("file id is: {0}", ctx.main_file.clone().expect("file info must exist").id);

    Ok(())
}

async fn ensure_server_pack(ctx: &mut Context) -> anyhow::Result<()> {
    let main_file = ctx.main_file
        .clone()
        .expect("main file must exist");

    if main_file.is_server_pack {
        if let Some(parent_id) = main_file.parent_project_file_id {
            let parent_file = ctx.client.get_file_info(ctx.project_id, parent_id)
                .await?;
            ctx.parent_file = Some(parent_file);
        }

        return Ok(());
    }

    if main_file.server_pack_file_id.is_none() {
        return Ok(());
    }

    ctx.parent_file = Some(main_file.clone());

    let file_id = main_file.server_pack_file_id
        .expect("server pack file id can't be none!");
    let main_file = ctx.client.get_file_info(ctx.project_id, file_id)
        .await?;

    ctx.main_file = Some(main_file);
    Ok(())
}

async fn setup(_ctx: &mut Context) -> anyhow::Result<()> {
    let dir = PathBuf::from(".mcsi");
    if !dir.exists() {
        std::fs::create_dir(dir)?;
    }

    Ok(())
}

async fn download_modpack(ctx: &mut Context) -> anyhow::Result<()> {
    download_client(ctx).await?;
    download_server(ctx).await?;

    resolve_mc_info(ctx).await?;

    let work_dir = work_dir();
    if work_dir.exists() {
        remove_dir_all(&work_dir)
            .await?;
    }
    create_dir(&work_dir)
        .await?;

    let server_path = PathBuf::from("./.mcsi")
        .join("server");
    let client_path = PathBuf::from("./.mcsi")
        .join("client");

    if ctx.parent_file.is_some() {
        // Extract server pack
        let server_files = get_closest_common_parent(&server_path)
            .await?;

        recursive_copy_to_dir(&server_files, work_dir.clone())
            .await?;
    } else {
        // Extract client pack
        let overrides = client_path
            .join("overrides");

        if overrides.is_dir() {
            recursive_copy_to_dir(overrides, work_dir.clone())
                .await?;
        }

        let mods_dir = PathBuf::from(work_dir.clone())
            .join("mods");
        if !mods_dir.is_dir() {
            create_dir_all(&mods_dir)
                .await?;
        }

        let Some(mod_list) = &ctx.mod_list.clone() else { return Err(FlameError::NoModList)? };
        for entry in mod_list {
            if !entry.required {
                continue;
            }

            let mod_info = ctx.client.get_mod_info(entry.project_id as u64)
                .await?;

            if mod_info.class_id != 6 {
                continue;
            }

            let info = ctx.client.get_file_info(entry.project_id as u64, entry.file_id as u64)
                .await?;

            let dst = PathBuf::from(mods_dir.clone())
                .join(info.file_name);

            download_file(&info.download_url, dst)
                .await?;
        }
    }

    if server_path.is_dir() {
        remove_dir_all(server_path)
            .await?;
    }
    if client_path.is_dir() {
        remove_dir_all(client_path)
            .await?;
    }

    match &ctx.mod_loader.clone().unwrap() {
        ModLoader::Forge { version } => {
            info!("loader version resolved to: {}", version);

            install_forge(ctx.mc_version.clone().unwrap(), version, &work_dir)
                .await?;

            info!("finished installing forge!");
        }
        ModLoader::Fabric { version } => {
            info!("loader version resolved to: {}", version);
            println!("Installing fabric");

            install_fabric(ctx.mc_version.clone().unwrap(), version, &work_dir)
                .await?;

            println!("done!");
        }
        ModLoader::Quilt { .. } => {}
    }

    Ok(())
}

async fn post_process(ctx: &mut Context) -> anyhow::Result<()> {
    info!("Finishing up...");
    let work_dir = work_dir();

    recursive_copy_to_dir(&work_dir, &ctx.target_dir)
        .await?;

    remove_dir_all(&work_dir)
        .await?;

    let mcsi_dir = ctx.target_dir
        .join(".mcsi");

    if !mcsi_dir.is_dir() {
        remove_dir_all(PathBuf::from("./.mcsi"))
            .await?;
        create_dir(&mcsi_dir)
            .await?;
    }

    let flame_manifest_path = mcsi_dir
        .join("flame.json");

    let pack_manifest = PackManifest::builder()
        .with_files_from_dir(&ctx.target_dir)
        .exclude_files_from_dir(".mcsi/")
        .finish();

    pack_manifest.save_to(flame_manifest_path)?;

    info!("Server is installed!");
    Ok(())
}

async fn resolve_mc_info(ctx: &mut Context) -> anyhow::Result<()> {
    let client_manifest_path = PathBuf::from("./.mcsi")
        .join("client")
        .join("manifest.json");
    let manifest_contents = std::fs::read_to_string(client_manifest_path)?;
    let flame_manifest: ClientManifest = serde_json::from_str(manifest_contents.as_str())?;

    let mc_version = McVersion::from_str(&flame_manifest.minecraft.version)?;
    ctx.mc_version = Some(mc_version);

    let primary_loader = &flame_manifest.minecraft.mod_loaders.iter().find(|loader| loader.primary).unwrap();
    let mod_loader = ModLoader::from_str(&primary_loader.id)?;
    ctx.mod_loader = Some(mod_loader);

    let mod_list = &flame_manifest.files;
    ctx.mod_list = Some(mod_list.to_owned());

    Ok(())
}


async fn download_client(ctx: &mut Context) -> anyhow::Result<()> {
    let client_file = if ctx.main_file.clone().is_some_and(|entry| !entry.is_server_pack) {
        ctx.main_file.clone().unwrap()
    } else if ctx.parent_file.clone().is_some_and(|entry| !entry.is_server_pack) {
        ctx.parent_file.clone().unwrap()
    } else {
        panic!()
    };

    let file_path = download_file(&client_file.download_url, PathBuf::from("./.mcsi/").join(client_file.file_name))
        .await?;
    {
        let file = File::open(&file_path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        let client_path = PathBuf::from("./.mcsi")
            .join("client");
        if client_path.exists() {
            remove_dir_all(&client_path)
                .await?;
        }
        create_dir(&client_path)
            .await?;
        archive.extract(&client_path)?;
    }

    remove_file(file_path)
        .await?;

    Ok(())
}

async fn download_server(ctx: &mut Context) -> anyhow::Result<()> {
    let server_pack = if ctx.main_file.clone().is_some_and(|entry| entry.is_server_pack) {
        ctx.main_file.clone().unwrap()
    } else {
        return Ok(())
    };

    let file_path = download_file(&server_pack.download_url, PathBuf::from("./.mcsi/").join(server_pack.file_name))
        .await?;
    {
        let file = File::open(&file_path)?;
        let mut archive = zip::ZipArchive::new(file)?;

        let server_path = PathBuf::from("./.mcsi")
            .join("server");
        if server_path.exists() {
            remove_dir_all(&server_path)
                .await?;
        }
        create_dir(&server_path)
            .await?;
        archive.extract(&server_path)?;
    }

    remove_file(file_path)
        .await?;

    Ok(())
}

#[derive(Error, Clone, Debug)]
pub enum FlameError {
    #[error("Client manifest has no mod list!")]
    NoModList,
}