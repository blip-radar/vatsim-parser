use std::{collections::HashMap, num::ParseIntError, str::FromStr};

use bevy_derive::Deref;
use pest::{iterators::Pair, Parser};
use pest_derive::Parser;
use serde::Serialize;
use tracing::warn;

use crate::adaptation::colours::Colour;

use super::{map::ColourDef, TopskyError};

#[derive(Parser)]
#[grammar = "pest/topsky_settings.pest"]
pub struct TopskySettingsParser;

enum Setting {
    Colour(ColourDef),
    Other(String, String),
}

#[derive(Clone, Debug, Serialize, Deref)]
pub struct Settings(pub HashMap<String, String>);
impl Settings {
    pub fn parse_with_default<T: FromStr>(&self, key: &str, default: T) -> T {
        self.0
            .get(key)
            .and_then(|font_str| {
                font_str.parse().map_or_else(
                    |_| {
                        warn!("Could not parse {key}");
                        None
                    },
                    Some,
                )
            })
            .unwrap_or(default)
    }
}

fn parse_colour(pair: Pair<Rule>) -> Result<Colour, ParseIntError> {
    let mut colour = pair.into_inner();
    let r = colour.next().unwrap().as_str().parse()?;
    let g = colour.next().unwrap().as_str().parse()?;
    let b = colour.next().unwrap().as_str().parse()?;
    Ok(Colour::from_rgb(r, g, b))
}

fn parse_setting(pair: Pair<Rule>) -> Option<Setting> {
    match pair.as_rule() {
        Rule::colour_setting => {
            let mut symbol = pair.into_inner();
            let name = symbol.next().unwrap().as_str().to_string();
            symbol.next().and_then(|rgb| match parse_colour(rgb) {
                Ok(colour) => Some(Setting::Colour(ColourDef { name, colour })),
                Err(e) => {
                    warn!("Could not parse colour {name}: {e:?}");
                    None
                }
            })
        }
        Rule::other_setting => {
            let mut setting = pair.into_inner();
            let name = setting.next().unwrap().as_str().to_string();
            let value = setting.next().unwrap().as_str().to_string();
            Some(Setting::Other(name, value))
        }
        Rule::section | Rule::EOI => None,
        rule => unreachable!("{rule:?}"),
    }
}

pub(super) fn parse_topsky_settings(
    contents: &str,
) -> Result<(HashMap<String, ColourDef>, Settings), TopskyError> {
    let colours = TopskySettingsParser::parse(Rule::settings, contents).map(|mut pairs| {
        pairs
            .next()
            .unwrap()
            .into_inner()
            .filter_map(parse_setting)
            .fold(
                (HashMap::new(), Settings(HashMap::new())),
                |(mut colours, Settings(mut settings)), setting| {
                    match setting {
                        Setting::Colour(colour) => {
                            colours.insert(colour.name.clone(), colour);
                        }
                        Setting::Other(key, val) => {
                            settings.insert(key, val);
                        }
                    }

                    (colours, Settings(settings))
                },
            )
    })?;

    Ok(colours)
}

#[cfg(test)]
mod test {
    use crate::{adaptation::colours::Colour, topsky::settings::parse_topsky_settings};

    #[test]
    fn test_settings_colours() {
        let settings_str = r"Setup_COOPANS=1

Airspace_ASSR_Type=2
Color_Active_Map_Type_16=160,160,160
Color_Active_Map_Type_17=255,255,255
Color_Active_Map_Type_18=0,160,0 //MVA - light gray
Color_Active_Map_Type_19=225,225,225 //Airspace C TMZ and AWYs
Color_Active_Map_Type_20=140,140,140 //Sectorlines and Labels
[_APP]
Color_Active_Map_Type_17=205,255,255
[EDMM_]
        ";
        let colours = parse_topsky_settings(settings_str).unwrap().0;

        assert!(
            colours.get("Active_Map_Type_16").unwrap().colour == Colour::from_rgb(160, 160, 160)
        );
        assert!(
            colours.get("Active_Map_Type_17").unwrap().colour == Colour::from_rgb(255, 255, 255)
        );
        assert!(colours.get("Active_Map_Type_18").unwrap().colour == Colour::from_rgb(0, 160, 0));
        assert!(
            colours.get("Active_Map_Type_19").unwrap().colour == Colour::from_rgb(225, 225, 225)
        );
        assert!(
            colours.get("Active_Map_Type_20").unwrap().colour == Colour::from_rgb(140, 140, 140)
        );
    }
}
