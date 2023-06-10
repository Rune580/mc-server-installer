use std::fmt::{Display, Formatter};
use std::str::FromStr;

pub struct McVersion {
    pub major: u8,
    pub minor: u8,
    pub patch: u8,
}

impl McVersion {

}

impl FromStr for McVersion {
    type Err = McVersionParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let nums = s.split(".");
        if nums.count() != 3 {
            Err(McVersionParseError::InvalidInput)
        }
    }
}

#[derive()]
pub enum  McVersionParseError {
    InvalidInput
}

impl Display for McVersionParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            McVersionParseError::InvalidInput => f.write_str("Invalid format")
        }
    }
}