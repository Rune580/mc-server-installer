use std::io::Write;
use std::path::{Path, PathBuf};
use futures_util::StreamExt;
use log::{debug, info};
use tokio::io::AsyncWriteExt;
use walkdir::WalkDir;

pub async fn recursive_copy_to_dir<TSrc: AsRef<Path>, TDst: AsRef<Path>>(src_dir: TSrc, dst_dir: TDst) -> anyhow::Result<()> {
    let src_dir = src_dir.as_ref();
    let dst_dir = dst_dir.as_ref().clone().to_str().unwrap();

    debug!("copying files from \'{src_dir:#?}\' to \'{dst_dir:#?}\'");

    let root = src_dir.clone().canonicalize()?;

    for entry in WalkDir::new(src_dir) {
        let entry = entry?;
        if entry.path().is_dir() {
            continue;
        }
        let relative = entry.path().clone().canonicalize()?;
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

    let total_bytes = resp.content_length();
    let mut bytes: u64 = 0;
    let mut last_increment = 0;

    let mut stream = resp.bytes_stream();
    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;

        bytes += chunk.len() as u64;

        file.write_all(&chunk).await?;

        match total_bytes {
            Some(total) => {
                let increment = bytes / (total / 20);
                if increment >= last_increment {
                    let percent = ((increment as f64 / 20f64) * 100f64).floor() as u64;
                    info!("Downloading {0}, {1} bytes of {2} received, {3}%", file_name, bytes, total, percent);
                    last_increment = increment + 1;
                }
            }
            None => {
                info!("{0} bytes received", bytes);
            }
        }
    }

    file.flush().await?;
    println!("Downloaded {0}", file_name);

    Ok(file_path.to_path_buf())
}