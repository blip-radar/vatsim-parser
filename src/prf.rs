use std::collections::HashMap;
use std::env::current_dir;
use std::ffi::OsStr;
use std::io;
use std::path::{Path, PathBuf};

use pest::{iterators::Pair, Parser};
use pest_derive::Parser;
use serde::Serialize;

use thiserror::Error;

use crate::TwoKeyMap;

use super::read_to_string;

#[derive(Error, Debug)]
pub enum PrfError {
    #[error("failed to read .prf file: {0}")]
    FileRead(#[from] io::Error),
    #[error("failed to parse .prf file: {0}")]
    Parse(#[from] pest::error::Error<Rule>),
}

#[derive(Parser)]
#[grammar = "pest/prf.pest"]
pub struct PrfParser;

#[derive(Clone, Debug, Serialize)]
pub struct Prf {
    settings: TwoKeyMap<String, String, String>,
    path: PathBuf,
}

pub type PrfResult = Result<Prf, PrfError>;

impl Prf {
    fn join_settings_path(&self, path: &str) -> PathBuf {
        let normalised_path = path.trim_end().replace('\\', "/");
        if let Some(rel_path) = normalised_path.strip_prefix('/') {
            self.path.parent().unwrap().join(rel_path)
        } else {
            current_dir().unwrap().join(normalised_path)
        }
    }
    pub fn sct_path(&self) -> PathBuf {
        self.join_settings_path(&self.settings.0[&("Settings".to_string(), "sector".to_string())])
    }

    pub fn ese_path(&self) -> PathBuf {
        self.join_settings_path(
            &self.settings.0[&("Settings".to_string(), "sector".to_string())]
                .replace(".sct", ".ese"),
        )
    }

    pub fn airways_path(&self) -> PathBuf {
        self.join_settings_path(&self.settings.0[&("Settings".to_string(), "airways".to_string())])
    }

    pub fn symbology_path(&self) -> PathBuf {
        self.join_settings_path(
            &self.settings.0[&("Settings".to_string(), "SettingsfileSYMBOLOGY".to_string())],
        )
    }

    pub fn squawks_path(&self) -> Option<PathBuf> {
        self.settings
            .0
            .iter()
            .find_map(|(_, v)| {
                let path = self.join_settings_path(v);
                if path.file_name() == Some(OsStr::new("Squawks.dll")) {
                    path.parent()
                        .map(Path::to_path_buf)
                        .map(|path| path.join("squawks.json"))
                } else {
                    None
                }
            })
            .map(|path| self.path.parent().unwrap().join(path))
    }

    pub fn topsky_path(&self) -> Option<PathBuf> {
        self.settings
            .0
            .iter()
            .find_map(|(_, v)| {
                let path = self.join_settings_path(v);
                if path.file_name() == Some(OsStr::new("TopSky.dll")) {
                    path.parent().map(Path::to_path_buf)
                } else {
                    None
                }
            })
            .map(|path| self.path.parent().unwrap().join(path))
    }

    pub fn recent_path(&self, num: u8) -> Option<PathBuf> {
        self.settings
            .0
            .get(&("RecentFiles".to_string(), format!("Recent{num}")))
            .map(|recent_path| self.join_settings_path(recent_path))
    }

    pub fn parse(path: &Path, contents: &[u8]) -> PrfResult {
        let file_contents = read_to_string(contents)?;
        let settings = PrfParser::parse(Rule::prf, &file_contents).map(|mut pairs| {
            pairs
                .next()
                .unwrap()
                .into_inner()
                .filter_map(parse_setting)
                .collect::<HashMap<_, _>>()
        })?;

        Ok(Prf {
            settings: TwoKeyMap(settings),
            path: path.canonicalize()?,
        })
    }
}

fn parse_setting(pair: Pair<Rule>) -> Option<((String, String), String)> {
    match pair.as_rule() {
        Rule::setting => {
            let mut setting = pair.into_inner();
            let category = setting.next().unwrap().as_str().to_string();
            let key = setting.next().unwrap().as_str().to_string();
            let value = setting.next().unwrap().as_str().to_string();
            Some(((category, key), value))
        }
        Rule::EOI => None,
        rule => unreachable!("{rule:?}"),
    }
}

#[cfg(test)]
mod test {
    use std::{fs, path::PathBuf};

    use super::Prf;

    #[test]
    fn test_basic_paths() {
        let prf_path = PathBuf::from("./fixtures/iCAS2.prf");
        let prf_contents = fs::read(&prf_path).unwrap();
        let prf = Prf::parse(&prf_path, &prf_contents).unwrap();

        assert_eq!(
            prf.symbology_path(),
            PathBuf::from(".")
                .canonicalize()
                .unwrap()
                .join("./fixtures/EDMM/Settings/iCAS2/Symbology.txt")
        );
        assert_eq!(
            prf.topsky_path(),
            Some(
                PathBuf::from(".")
                    .canonicalize()
                    .unwrap()
                    .join("fixtures/EDMM/Plugins/Topsky/iCAS2")
            )
        );
        assert_eq!(
            prf.sct_path(),
            PathBuf::from(".")
                .canonicalize()
                .unwrap()
                .join("./fixtures/EDMM-AeroNav.sct")
        );
    }

    #[test]
    fn test_recent_path() {
        let prf_path = PathBuf::from("./fixtures/iCAS2.prf");
        let prf_contents = fs::read(&prf_path).unwrap();
        let prf = Prf::parse(&prf_path, &prf_contents).unwrap();

        assert_eq!(
            prf.recent_path(1),
            Some(
                PathBuf::from(".")
                    .canonicalize()
                    .unwrap()
                    .join("./fixtures/EDMM/ASR/iCAS2/EDMM_CTR.asr")
            )
        );
        assert_eq!(prf.recent_path(2), None);
    }
}
