use std::fmt::{Display, Formatter};
use std::str::FromStr;
use thiserror::Error;

#[derive(Clone, Debug)]
pub struct McVersion {
    pub major: u8,
    pub minor: u8,
    pub patch: u8,
}

impl McVersion {
    pub fn as_str(&self) -> String {
        format!("{0}.{1}.{2}", self.major, self.minor, self.patch)
    }
}

impl FromStr for McVersion {
    type Err = McVersionParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let nums = s.split(".");
        if nums.clone().count() != 3 {
            return Err(McVersionParseError::InvalidInput);
        }

        let nums: Vec<u8> = nums
            .map(|entry| u8::from_str(entry).unwrap())
            .collect();

        Ok(McVersion {
            major: nums[0],
            minor: nums[1],
            patch: nums[2]
        })
    }
}

#[derive(Error, Clone, Debug)]
pub enum McVersionParseError {
    #[error("Input is invalid")]
    InvalidInput,
}