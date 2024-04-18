use std::{collections::HashMap, fs, path::PathBuf};

use pest::{iterators::Pair, Parser};
use pest_derive::Parser;

use crate::{read_to_string, Color};

use super::{map::ColorDef, TopskyError};

#[derive(Parser)]
#[grammar = "topsky/settings.pest"]
pub struct TopskySettingsParser;

fn parse_setting(pair: Pair<Rule>) -> Option<ColorDef> {
    match pair.as_rule() {
        Rule::color_setting => {
            let mut symbol = pair.into_inner();
            let name = symbol.next().unwrap().as_str().to_string();
            symbol.next().map(|rgb| {
                let mut color = rgb.into_inner();

                let r = color.next().unwrap().as_str().parse().unwrap();
                let g = color.next().unwrap().as_str().parse().unwrap();
                let b = color.next().unwrap().as_str().parse().unwrap();
                ColorDef {
                    name,
                    color: Color::from_rgb(r, g, b),
                }
            })
        }
        Rule::ignored_setting | Rule::EOI => None,
        rule => {
            eprintln!("{rule:?}");
            unreachable!()
        }
    }
}

pub(super) fn parse_topsky_settings(
    path: PathBuf,
) -> Result<HashMap<String, ColorDef>, TopskyError> {
    let file_contents = read_to_string(&fs::read(path)?)?;
    let colors = TopskySettingsParser::parse(Rule::settings, &file_contents).map(|mut pairs| {
        pairs
            .next()
            .unwrap()
            .into_inner()
            .filter_map(parse_setting)
            .map(|color| (color.name.clone(), color))
            .collect::<HashMap<_, _>>()
    })?;

    Ok(colors)
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use pest::Parser;

    use crate::Color;

    use super::{parse_setting, Rule, TopskySettingsParser};

    #[test]
    fn test_settings_colors() {
        let settings_str = r"
Color_Active_Map_Type_16=160,160,160
Color_Active_Map_Type_17=255,255,255
Color_Active_Map_Type_18=0,160,0 //MVA - light gray
Color_Active_Map_Type_19=225,225,225 //Airspace C TMZ and AWYs
Color_Active_Map_Type_20=140,140,140 //Sectorlines and Labels
        ";
        let colors = TopskySettingsParser::parse(Rule::settings, settings_str).map(|mut pairs| {
            pairs
                .next()
                .unwrap()
                .into_inner()
                .filter_map(parse_setting)
                .map(|color| (color.name.clone(), color))
                .collect::<HashMap<_, _>>()
        });

        assert!(
            colors
                .as_ref()
                .unwrap()
                .get("Active_Map_Type_16")
                .unwrap()
                .color
                == Color::from_rgb(160, 160, 160)
        );
        assert!(
            colors
                .as_ref()
                .unwrap()
                .get("Active_Map_Type_17")
                .unwrap()
                .color
                == Color::from_rgb(255, 255, 255)
        );
        assert!(
            colors
                .as_ref()
                .unwrap()
                .get("Active_Map_Type_18")
                .unwrap()
                .color
                == Color::from_rgb(0, 160, 0)
        );
        assert!(
            colors
                .as_ref()
                .unwrap()
                .get("Active_Map_Type_19")
                .unwrap()
                .color
                == Color::from_rgb(225, 225, 225)
        );
        assert!(
            colors
                .as_ref()
                .unwrap()
                .get("Active_Map_Type_20")
                .unwrap()
                .color
                == Color::from_rgb(140, 140, 140)
        );
    }
}
