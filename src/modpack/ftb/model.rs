use serde::Deserialize;

#[derive(Deserialize, Clone, Debug)]
pub struct SearchResults {
    pub packs: Vec<usize>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct PackDetails {
    pub versions: Vec<PackVersion>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct PackVersion {
    pub specs: Specs,
    pub targets: Vec<Target>,
    pub id: usize,
    pub name: String,
    #[serde(rename = "type")]
    pub version_type: String,
    pub updated: u64,
    pub private: bool,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Specs {
    pub id: usize,
    pub minimum: usize,
    pub recommended: usize,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Target {
    pub version: String,
    pub id: usize,
    pub name: String,
    #[serde(rename = "type")]
    pub target_type: String,
    pub updated: u64,
}