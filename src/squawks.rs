use serde::{Deserialize, Serialize};
use std::io;
use thiserror::Error;

use crate::adaptation::colours::Colour;

#[derive(Error, Debug)]
pub enum SquawksError {
    #[error("failed to read squawks.json: {0}")]
    FileRead(#[from] io::Error),
    #[error("failed to deserialize squawks.json: {0}")]
    Deserialize(#[from] serde_json::Error),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Range {
    pub start: String,
    pub end: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SquawkDef {
    pub code: Option<String>,
    pub range: Option<Range>,
    pub message: String,
    pub color: Colour,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all(deserialize = "PascalCase"))]
pub struct SquawksJson {
    pub non_discrete_color: Colour,
    pub non_discrete_message: String,
    pub non_discrete: Vec<String>,
    pub squawks: Vec<SquawkDef>,
}
pub type SquawksResult = Result<SquawksJson, SquawksError>;

pub fn parse_squawks_json(content: &[u8]) -> SquawksResult {
    Ok(serde_json::from_slice(content)?)
}
