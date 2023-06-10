use std::str::FromStr;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod fabric;

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
            Ok(ModLoader::Forge { version: s.clone().to_string() })
        } else if s.contains("fabric") {
            let fabric_version = s.clone()
                .split("-")
                .last()
                .ok_or(ModLoaderParseError::InvalidInput)?;

            Ok(ModLoader::Fabric {
                version: fabric_version.to_string()
            })
        } else if s.contains("quilt") {
            Ok(ModLoader::Quilt { version: s.clone().to_string() })
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