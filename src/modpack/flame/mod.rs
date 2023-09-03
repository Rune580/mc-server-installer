use std::path::PathBuf;
use std::str::FromStr;
use log::{debug, info};
use reqwest::{Client, header::HeaderMap};
use crate::fs_utils::download_file;
use crate::modloader::fabric::install_fabric;
use crate::modloader::forge::install_forge;
use crate::modloader::ModLoader;
use crate::modpack::flame::model::{ClientManifest, FileEntry};
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
}

pub async fn handle_flame(
    api_key: String,
    project_id: u64,
    version: String
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
    };

    setup(&mut ctx).await?;
    resolve_main_file(&mut ctx).await?;
    ensure_server_pack(&mut ctx).await?;
    download_modpack(&mut ctx).await?;

    Ok(())
}

async fn resolve_main_file(ctx: &mut Context) -> anyhow::Result<()> {
    if ctx.version.eq_ignore_ascii_case("latest") {
        info!("Version set to \'latest\', determining file id...");

        let info = ctx.client.get_modpack_info(ctx.project_id)
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

async fn setup(ctx: &mut Context) -> anyhow::Result<()> {
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

    let work_dir = PathBuf::from("./.mcsi")
        .join("work_dir");
    if work_dir.exists() {
        std::fs::remove_dir_all(&work_dir)?;
    }
    std::fs::create_dir(&work_dir)?;

    if ctx.parent_file.is_some() {
        let server_path = PathBuf::from("./.mcsi")
            .join("server");

        crate::fs_utils::recursive_copy_to_dir(&server_path, work_dir.clone())
            .await?;

        match &ctx.mod_loader.clone().unwrap() {
            ModLoader::Forge { version } => {
                info!("loader version resolved to: {}", version);

                install_forge(ctx.mc_version.clone().unwrap(), version, &work_dir)
                    .await?;
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
    }

    Ok(())
}

async fn resolve_mc_info(ctx: &mut Context) -> anyhow::Result<()> {
    let flame_manifest_path = PathBuf::from("./.mcsi")
        .join("client")
        .join("manifest.json");
    let manifest_contents = std::fs::read_to_string(flame_manifest_path)?;
    let flame_manifest: ClientManifest = serde_json::from_str(manifest_contents.as_str())?;

    let mc_version = McVersion::from_str(&flame_manifest.minecraft.version)?;
    ctx.mc_version = Some(mc_version);

    let primary_loader = &flame_manifest.minecraft.mod_loaders.iter().find(|loader| loader.primary).unwrap();
    let mod_loader = ModLoader::from_str(&primary_loader.id)?;
    ctx.mod_loader = Some(mod_loader);

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
    let file = std::fs::File::open(file_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    let client_path = PathBuf::from("./.mcsi")
        .join("client");
    if client_path.exists() {
        std::fs::remove_dir_all(&client_path)?;
    }
    std::fs::create_dir(&client_path)?;
    archive.extract(&client_path)?;

    Ok(())
}

async fn download_server(ctx: &mut Context) -> anyhow::Result<()> {
    let server_pack = if ctx.main_file.clone().is_some_and(|entry| entry.is_server_pack) {
        ctx.main_file.clone().unwrap()
    } else {
        return Ok(())
    };

    let file_pack = download_file(&server_pack.download_url, PathBuf::from("./.mcsi/").join(server_pack.file_name))
        .await?;
    let file = std::fs::File::open(file_pack)?;
    let mut archive = zip::ZipArchive::new(file)?;

    let server_path = PathBuf::from("./.mcsi")
        .join("server");
    if server_path.exists() {
        std::fs::remove_dir_all(&server_path)?;
    }
    std::fs::create_dir(&server_path)?;
    archive.extract(&server_path)?;

    Ok(())
}