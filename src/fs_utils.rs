use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use chrono::Utc;
use futures_util::StreamExt;
use indicatif::ProgressBar;
use log::warn;
use tokio::fs::{create_dir_all, read, remove_dir, remove_file, write};
use tokio::io::AsyncWriteExt;
use walkdir::WalkDir;
use crate::cli;

pub async fn recursive_copy_to_dir<TSrc: AsRef<Path>, TDst: AsRef<Path>>(src_dir: TSrc, dst_dir: TDst) -> anyhow::Result<()> {
    let src_dir = src_dir.as_ref();
    let dst_dir = dst_dir.as_ref().to_str().unwrap();

    println!("Copying files...");
    let copy_bar = ProgressBar::new_spinner()
        .with_style(cli::copy_progress_style());

    let root = src_dir.canonicalize()?;
    let mut count = 0;
    for entry in WalkDir::new(src_dir) {
        let entry = entry?;
        if entry.path().is_dir() {
            continue;
        }
        let relative = entry.path().canonicalize()?;
        let mut relative = relative.to_str().unwrap().replace(root.to_str().unwrap(), "");
        if relative.starts_with('/') || relative.starts_with('\\') {
            relative.remove(0);
        }
        let relative = PathBuf::from(relative);

        let dst = PathBuf::from(dst_dir)
            .join(&relative);

        if let Some(parent) = dst.parent() {
            if !parent.exists() {
                create_dir_all(parent)
                    .await?;
            }
        }

        count += 1;
        copy_bar.set_prefix(format!("[{}/?]", count));
        copy_bar.set_message(String::from(relative.to_str().unwrap()));

        if dst.is_file() {
            std::fs::remove_file(&dst)?;
        }

        let mut dst_file = File::create(dst)?;
        let bytes = std::fs::read(entry.path())?;

        dst_file.write_all(&bytes)?;
    }

    copy_bar.finish();

    Ok(())
}

pub async fn backup_and_remove_files<TSrc: AsRef<Path>, TDst: AsRef<Path>>(
    src_dir: TSrc,
    backup_dir: TDst,
    rel_files: Vec<String>,
) -> anyhow::Result<()> {
    let src_dir = src_dir.as_ref();
    let backup_dir = backup_dir.as_ref();

    if !backup_dir.is_dir() {
        create_dir_all(&backup_dir)
            .await?;
    }

    println!("Starting backup of files...");
    let backup_bar = ProgressBar::new(rel_files.len() as u64)
        .with_style(cli::backup_progress_style());

    for rel_file in rel_files {
        let src_file = PathBuf::from(&src_dir)
            .join(&rel_file);
        let dst_file = PathBuf::from(&backup_dir)
            .join(&rel_file);

        if !src_file.is_file() {
            warn!("File {} doesn't exist!", &src_file.to_str().unwrap());
            continue;
        }

        backup_bar.set_message(rel_file);

        ensure_parent(&dst_file)
            .await?;

        let bytes = read(&src_file)
            .await?;
        write(dst_file, bytes)
            .await?;

        if let Some(parent_dir) = src_file.parent() {
            let src_parent_dir = parent_dir.to_path_buf();

            remove_file(&src_file)
                .await?;

            let children = WalkDir::new(&src_parent_dir)
                .max_depth(1)
                .into_iter()
                .count();

            if children == 0 {
                remove_dir(src_parent_dir)
                    .await?;
            }
        } else {
            remove_file(&src_file)
                .await?;
        }

        backup_bar.inc(1);
    }

    backup_bar.finish();

    Ok(())
}

pub fn file_path_relative_to<TFile: AsRef<Path>, TDir: AsRef<Path>>(file: TFile, dir: TDir) -> anyhow::Result<PathBuf> {
    let file = file.as_ref();
    let dir = dir.as_ref();

    let relative = file.canonicalize()?;
    let mut relative = relative.to_str().unwrap().replace(dir.canonicalize()?.to_str().unwrap(), "");
    if relative.starts_with('/') || relative.starts_with('\\') {
        relative.remove(0);
    }
    let relative = PathBuf::from(relative);
    
    Ok(relative)
}

pub async fn download_file<T: AsRef<Path>>(url: &str, dst: T) -> anyhow::Result<PathBuf> {
    let file_name = dst.as_ref().file_name().unwrap().to_str().unwrap().to_string();
    let file_path = dst.as_ref();

    if file_path.is_file() {
        std::fs::remove_file(file_path)?;
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
            item.siblings = *siblings.get(&item.depth).unwrap();
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
        std::fs::create_dir_all(dir)?;
    }

    Ok(())
}

pub async fn ensure_parent<T: AsRef<Path>>(path: T) -> anyhow::Result<()> {
    let path = path.as_ref();

    if let Some(parent) = path.parent() {
        if !parent.is_dir() {
            create_dir_all(parent)
                .await?;
        }
    }

    Ok(())
}

pub fn mcsi_dir() -> PathBuf {
    PathBuf::from("./.mcsi")
}

pub fn work_dir() -> PathBuf {
    mcsi_dir().join("work_dir")
}

pub fn logs_dir() -> PathBuf {
    mcsi_dir().join("logs")
}

pub fn get_log_file() -> anyhow::Result<File> {
    let logs_dir = logs_dir();
    if !logs_dir.is_dir() {
        std::fs::create_dir_all(&logs_dir)?;
    }

    let now = Utc::now().to_rfc3339();
    let file_name = format!("{now}.log").replace(':', "");
    let log_file_path = logs_dir.join(file_name);

    let file = File::create(log_file_path)?;

    Ok(file)
}