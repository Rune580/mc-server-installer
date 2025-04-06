use reqwest::Client;
use serde::de::DeserializeOwned;
use crate::modpack::flame::model::{FileEntry, FilesList};

use super::model::ModInfo;

#[derive(Clone, Debug)]
pub struct FlameClient {
    client: Client,
}

impl FlameClient {
    pub fn new(client: Client) -> Self {
        FlameClient {
            client,
        }
    }

    pub async fn get_mod_info(&mut self, project_id: u64) -> color_eyre::Result<ModInfo> {
        let url = format!("https://api.curseforge.com/v1/mods/{0}", project_id);
        let resp = self.client.get(url)
            .send().await?
            .text().await?;

        let info: ModInfo = data_root(resp)?;

        Ok(info)
    }

    pub async fn get_file_info(&mut self, project_id: u64, file_id: u64) -> color_eyre::Result<FileEntry> {
        let url = format!("https://api.curseforge.com/v1/mods/{0}/files/{1}", project_id, file_id);
        let resp = self.client.get(url)
            .send().await?
            .text().await?;

        let file_info: FileEntry = data_root(resp)?;

        Ok(file_info)
    }

    // TODO: pagination support.
    pub async fn get_files(&mut self, project_id: u64, _page: u32) -> color_eyre::Result<FilesList> {
        let url = format!("https://api.curseforge.com/v1/mods/{0}/files", project_id);
        let resp = self.client.get(url)
            .send().await?
            .text().await?;

        let files: FilesList = data_root(resp)?;

        Ok(files)
    }
}

fn data_root<T: DeserializeOwned>(resp: String) -> color_eyre::Result<T> {
    let json: serde_json::Value = serde_json::from_str(resp.as_str())?;
    let root: T = serde_json::from_value(json.get("data").expect("root data").clone())?;

    Ok(root)
}