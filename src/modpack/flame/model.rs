use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct ModInfo {
    pub id: u64,
    #[serde(rename = "mainFileId")]
    pub main_file_id: u64,
    #[serde(rename = "latestFiles")]
    pub latest_files: Vec<FileEntry>,
    #[serde(rename = "classId")]
    pub class_id: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct FileEntry {
    pub id: u64,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "fileName")]
    pub file_name: String,
    #[serde(rename = "downloadUrl")]
    pub download_url: String,
    #[serde(rename = "isServerPack")]
    pub is_server_pack: bool,
    #[serde(rename = "serverPackFileId")]
    pub server_pack_file_id: Option<u64>,
    #[serde(rename = "parentProjectFileId")]
    pub parent_project_file_id: Option<u64>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct FilesList {
    #[serde(rename = "data")]
    pub files: Vec<FileEntry>,
    pub pagination: Pagination,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Pagination {
    pub index: u32,
    #[serde(rename = "pageSize")]
    pub page_size: u32,
    #[serde(rename = "resultCount")]
    pub result_count: u32,
    #[serde(rename = "totalCount")]
    pub total_count: u32,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ClientManifest {
    pub minecraft: ManifestMcInfo,
    pub files: Vec<ManifestFileEntry>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ManifestMcInfo {
    pub version: String,
    #[serde(rename = "modLoaders")]
    pub mod_loaders: Vec<ManifestModLoader>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ManifestModLoader {
    pub id: String,
    pub primary: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ManifestFileEntry {
    #[serde(rename = "projectID")]
    pub project_id: u32,
    #[serde(rename = "fileID")]
    pub file_id: u32,
    pub required: bool,
}