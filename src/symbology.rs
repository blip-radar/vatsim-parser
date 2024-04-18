use std::collections::HashMap;
use std::io;

use pest::{iterators::Pair, Parser};
use pest_derive::Parser;
use serde::Serialize;
use thiserror::Error;

use crate::TwoKeyMap;

use super::{read_to_string, Color};

#[derive(Parser)]
#[grammar = "symbology.pest"]
pub struct SymbologyParser;

#[derive(Error, Debug)]
pub enum SymbologyError {
    #[error("failed to parse .sct file: {0:?}")]
    Parse(#[from] pest::error::Error<Rule>),
    #[error("failed to read .sct file: {0:?}")]
    FileRead(#[from] io::Error),
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Item {
    pub folder: String,
    pub name: String,
    pub color: Color,
    pub font_size: f64,
    // TODO
    // line_weight,
    // line_style,
    // text_alignment,
}

#[derive(Debug, Clone, Serialize)]
pub struct Symbology {
    pub items: TwoKeyMap<String, String, Item>, // TODO symbols
}

pub type SymbologyResult = Result<Symbology, SymbologyError>;

impl Item {
    fn parse(pair: Pair<Rule>) -> Self {
        let mut item = pair.into_inner();
        let folder = item.next().unwrap().as_str().to_string();
        let name = item.next().unwrap().as_str().to_string();
        let color_str = item.next().unwrap().as_str();
        let color_num = color_str.parse::<i32>().unwrap();
        let font_size = item.next().unwrap().as_str().parse().unwrap();

        Self {
            folder,
            name,
            color: Color::from_euroscope(color_num),
            font_size,
        }
    }
}

impl Symbology {
    pub fn parse(content: &[u8]) -> SymbologyResult {
        let unparsed_file = read_to_string(content)?;
        let items = SymbologyParser::parse(Rule::symbology, &unparsed_file).map(|mut pairs| {
            pairs
                .next()
                .unwrap()
                .into_inner()
                .filter_map(|pair| match pair.as_rule() {
                    Rule::item => {
                        let item = Item::parse(pair);
                        Some(((item.folder.clone(), item.name.clone()), item))
                    }
                    Rule::header | Rule::symbols | Rule::EOI => None,
                    rule => {
                        eprintln!("unhandled {rule:?}");
                        unreachable!()
                    }
                })
                .collect::<HashMap<_, _>>()
        })?;

        Ok(Symbology {
            items: TwoKeyMap(items),
        })
    }
}

#[cfg(test)]
mod test {

    use crate::{
        symbology::{Item, Symbology},
        Color,
    };

    #[test]
    fn test_symbology() {
        let symbology_bytes = br"
SYMBOLOGY
SYMBOLSIZE
Sector:msaw:32768:2.0:0:2:7
Sector:inactive sector background:13158600:3.5:0:0:7
SYMBOL:0
SYMBOLITEM:MOVETO -3 -3
SYMBOLITEM:LINETO 3 -3
SYMBOLITEM:LINETO 3 3
SYMBOLITEM:LINETO -3 3
SYMBOLITEM:LINETO -3 -3
SYMBOLITEM:MOVETO 5 0
SYMBOLITEM:LINETO -6 0
SYMBOLITEM:MOVETO 0 5
SYMBOLITEM:LINETO 0 -6
SYMBOL:1
SYMBOLITEM:MOVETO -4 3
SYMBOLITEM:LINETO 0 -4
SYMBOLITEM:LINETO 4 3
SYMBOLITEM:LINETO -4 3
        ";
        let symbology = Symbology::parse(symbology_bytes);
        assert_eq!(
            symbology
                .as_ref()
                .unwrap()
                .items
                .0
                .get(&("Sector".to_string(), "msaw".to_string())),
            Some(&Item {
                folder: "Sector".to_string(),
                name: "msaw".to_string(),
                color: Color::from_rgb(0, 128, 0),
                font_size: 2.0
            })
        );
        assert_eq!(
            symbology.as_ref().unwrap().items.0.get(&(
                "Sector".to_string(),
                "inactive sector background".to_string()
            )),
            Some(&Item {
                folder: "Sector".to_string(),
                name: "inactive sector background".to_string(),
                color: Color::from_rgb(200, 200, 200),
                font_size: 3.5
            })
        );
    }
}
