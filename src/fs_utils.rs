use std::io::Write;
use std::path::{Path, PathBuf};
use futures_util::StreamExt;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeekExt};
use tokio::io::AsyncWriteExt;
use walkdir::WalkDir;

pub async fn recursive_copy_to_dir<TSrc: AsRef<Path>, TDst: AsRef<Path>>(src_dir: TSrc, dst_dir: TDst) -> anyhow::Result<()> {
    let src_dir = src_dir.as_ref();
    let dst_dir = dst_dir.as_ref();

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
        }
        let relative = PathBuf::from(relative);

        let dst = PathBuf::from(&dst_dir)
            .join(relative);

        if let Some(parent) = dst.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }

        println!("Copy from {0} to {1}", entry.path().display(), dst.display());

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

    let mut stream = reqwest::get(url)
        .await?
        .bytes_stream();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
        file.write_all(&chunk).await?;
    }

    file.flush().await?;
    println!("Downloaded {0}", file_name);

    Ok(file_path.to_path_buf())
}