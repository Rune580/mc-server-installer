use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FlameManifest {
    pub(crate) files: Vec<String>,
}