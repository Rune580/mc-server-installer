use serde::{Deserialize, Serialize};

pub mod fabric;

#[derive(Eq, PartialEq, Copy, Clone, Debug, Serialize, Deserialize)]
pub enum ModLoader {
    Forge,
    Fabric,
    Quilt,
}