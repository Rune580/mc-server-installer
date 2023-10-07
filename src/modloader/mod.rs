use std::str::FromStr;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod fabric;
pub mod forge;

#[derive(Eq, PartialEq, Clone, Debug, Serialize, Deserialize)]
pub enum ModLoader {
    Forge {
        version: String,
    },
    Fabric {
        version: String,
    },
    Quilt {
        version: String,
    },
}

impl FromStr for ModLoader {
    type Err = ModLoaderParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.contains("forge") {
            let forge_version = s
                .split("-")
                .last()
                .ok_or(ModLoaderParseError::InvalidInput)?;

            Ok(ModLoader::Forge {
                version: forge_version.to_string()
            })
        } else if s.contains("fabric") {
            let fabric_version = s
                .split("-")
                .last()
                .ok_or(ModLoaderParseError::InvalidInput)?;

            Ok(ModLoader::Fabric {
                version: fabric_version.to_string()
            })
        } else if s.contains("quilt") {
            Ok(ModLoader::Quilt { version: s.to_string() })
        } else {
            Err(ModLoaderParseError::InvalidInput)
        }
    }
}

#[derive(Error, Clone, Debug)]
pub enum ModLoaderParseError {
    #[error("Failed to parse mod loader")]
    InvalidInput,
}