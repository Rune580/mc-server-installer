use std::collections::HashMap;
use std::fs::create_dir_all;
use std::io::Write;
use std::path::{Path, PathBuf};
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use log::{debug, info};
use tokio::io::AsyncWriteExt;
use walkdir::WalkDir;
use crate::cli;

pub async fn recursive_copy_to_dir<TSrc: AsRef<Path>, TDst: AsRef<Path>>(src_dir: TSrc, dst_dir: TDst) -> anyhow::Result<()> {
    let src_dir = src_dir.as_ref();
    let dst_dir = dst_dir.as_ref().to_str().unwrap();

    debug!("copying files from \'{src_dir:#?}\' to \'{dst_dir:#?}\'");

    let root = src_dir.canonicalize()?;

    for entry in WalkDir::new(src_dir) {
        let entry = entry?;
        if entry.path().is_dir() {
            continue;
        }
        let relative = entry.path().canonicalize()?;
        let mut relative = relative.to_str().unwrap().replace(root.to_str().unwrap(), "");
        if relative.starts_with("/") {
            relative.remove(0);
        } else if relative.starts_with("\\") {
            relative.remove(0);
        }
        let relative = PathBuf::from(relative);

        let dst = PathBuf::from(dst_dir)
            .join(relative);

        if let Some(parent) = dst.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }

        debug!("Copy file from {0} to {1}", entry.path().display(), dst.display());

        if dst.is_file() {
            std::fs::remove_file(&dst)?;
        }

        let mut dst_file = std::fs::File::create(dst)?;
        let bytes = std::fs::read(entry.path())?;

        dst_file.write_all(&bytes)?;
    }

    Ok(())
}

pub fn file_path_relative_to<TFile: AsRef<Path>, TDir: AsRef<Path>>(file: TFile, dir: TDir) -> anyhow::Result<PathBuf> {
    let file = file.as_ref();
    let dir = dir.as_ref();

    let relative = file.canonicalize()?;
    let mut relative = relative.to_str().unwrap().replace(dir.canonicalize()?.to_str().unwrap(), "");
    if relative.starts_with("/") {
        relative.remove(0);
    } else if relative.starts_with("\\") {
        relative.remove(0);
    }
    let relative = PathBuf::from(relative);
    
    Ok(relative)
}

pub async fn download_file<T: AsRef<Path>>(url: &str, dst: T) -> anyhow::Result<PathBuf> {
    let file_name = dst.as_ref().file_name().unwrap().to_str().unwrap().to_string();
    let file_path = dst.as_ref();

    if file_path.is_file() {
        std::fs::remove_file(&file_path)?;
    }

    let mut file = tokio::fs::File::create(&file_path).await?;
    println!("Downloading {0}...", &file_name);

    let resp = reqwest::get(url)
        .await?;

    let total_bytes = resp.content_length().unwrap();
    let download_bar = ProgressBar::new(total_bytes)
        .with_style(cli::download_progress_style())
        .with_message(file_name.clone());

    let mut stream = resp.bytes_stream();
    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
        let chunk_len = chunk.len() as u64;


        file.write_all(&chunk).await?;

        download_bar.inc(chunk_len);
    }

    file.flush().await?;

    download_bar.finish();

    Ok(file_path.to_path_buf())
}

struct DirDepthEntry {
    depth: usize,
    siblings: usize,
    path: PathBuf,
}

pub async fn get_closest_common_parent<T: AsRef<Path>>(dir: T) -> anyhow::Result<PathBuf> {
    let dir = dir.as_ref();
    let mut items: Vec<DirDepthEntry> = Vec::new();
    let mut siblings: HashMap<usize, usize> = HashMap::new();

    for entry in WalkDir::new(dir) {
        let entry = entry?;
        if !entry.path().is_dir() {
            continue;
        }

        let item = DirDepthEntry {
            depth: entry.depth(),
            siblings: 0,
            path: entry.path().to_path_buf(),
        };

        items.push(item);
        siblings.entry(entry.depth())
            .and_modify(|val| *val += 1)
            .or_insert(0);
    }

    let common = items.iter_mut()
        .map(|item| {
            item.siblings = siblings.get(&item.depth).unwrap().clone();
            item
        })
        .filter(|item| item.siblings == 0)
        .max_by_key(|item| item.depth)
        .unwrap();

    let common_dir = PathBuf::from(&common.path);

    Ok(common_dir)
}

pub fn ensure_dir<T: AsRef<Path>>(dir: T) -> anyhow::Result<()> {
    let dir = dir.as_ref();

    if !dir.is_dir() {
        create_dir_all(dir)?;
    }

    Ok(())
}

pub fn work_dir() -> PathBuf {
    PathBuf::from("./.mcsi").join("work_dir")
}