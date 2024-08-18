use pest::Parser;
use pest_derive::Parser;
use std::collections::HashMap;
use std::io;
use thiserror::Error;

use crate::adaptation::icao::{Aircraft, AircraftType, EngineType, Wtc};

use super::read_to_string;

#[derive(Parser)]
#[grammar = "pest/base.pest"]
#[grammar = "pest/icao_aircraft.pest"]
pub struct AircraftParser;

#[derive(Error, Debug)]
pub enum AircraftError {
    #[error("failed to parse ICAO_Aircraft.txt: {0}")]
    Parse(#[from] pest::error::Error<Rule>),
    #[error("failed to read ICAO_Aircraft.txt: {0}")]
    FileRead(#[from] io::Error),
}

pub type AircraftResult = Result<HashMap<String, Aircraft>, AircraftError>;

pub fn parse_aircraft(content: &[u8]) -> AircraftResult {
    let unparsed_file = read_to_string(content)?;
    let aircraft_parse = AircraftParser::parse(Rule::aircraft, &unparsed_file);

    Ok(aircraft_parse.map(|mut pairs| {
        pairs
            .next()
            .unwrap()
            .into_inner()
            .fold(HashMap::new(), |mut acc, pair| {
                if matches!(pair.as_rule(), Rule::definition) {
                    let mut line = pair.into_inner();
                    let designator = line.next().unwrap().as_str().to_string();
                    let wtc = Wtc::parse(line.next().unwrap().as_str());
                    let aircrafttype = AircraftType::parse(line.next().unwrap().as_str());
                    let num_engines = line.next().unwrap().as_str().parse::<u8>().unwrap_or(0);
                    let enginetype = EngineType::parse(line.next().unwrap().as_str());
                    let manufacturer = line.next().unwrap().as_str().to_string();
                    let name = line.next().unwrap().as_str().to_string();

                    acc.entry(designator.clone()).or_insert(Aircraft {
                        designator,
                        wtc,
                        aircrafttype,
                        num_engines,
                        enginetype,
                        manufacturer,
                        name,
                    });
                }

                acc
            })
    })?)
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use pretty_assertions_sorted::assert_eq_sorted;

    use crate::adaptation::icao::{Aircraft, AircraftType, EngineType, Wtc};

    use super::parse_aircraft;

    #[test]
    fn test_aircraft() {
        let aircraft_bytes = b"
;========================COMMENT=======================================================================================;
EUFI	ML2J	ALENIA	Eurofighter 2000, Typhoon
F260	LL1P	AERMACCHI	SF260E/F, Warrior
A388	JL4J	AIRBUS	A380-800
C208	LL1T	CESSNA	208 Caravan 1, (Super)Cargomaster, Grand  Caravan (C98, U27)
EC45	LH2T	EUROCOPTER	EC145
C08T	LLCT	SOLOY	208 DualPac Caravan, Pathfinder 21
continued model name from the previous line we ignore
ZZZZ	----	-	Aircraft type not assigned
";

        let parsed = parse_aircraft(aircraft_bytes).unwrap();

        assert_eq_sorted!(
            parsed,
            HashMap::from([
                (
                    "EUFI".to_string(),
                    Aircraft {
                        designator: "EUFI".to_string(),
                        wtc: Wtc::MEDIUM,
                        aircrafttype: AircraftType::LANDPLANE,
                        num_engines: 2,
                        enginetype: EngineType::JET,
                        manufacturer: "ALENIA".to_string(),
                        name: "Eurofighter 2000, Typhoon".to_string(),
                    }
                ),
                (
                    "A388".to_string(),
                    Aircraft {
                        designator: "A388".to_string(),
                        wtc: Wtc::SUPER,
                        aircrafttype: AircraftType::LANDPLANE,
                        num_engines: 4,
                        enginetype: EngineType::JET,
                        manufacturer: "AIRBUS".to_string(),
                        name: "A380-800".to_string(),
                    }
                ),
                (
                    "C208".to_string(),
                    Aircraft {
                        designator: "C208".to_string(),
                        wtc: Wtc::LIGHT,
                        aircrafttype: AircraftType::LANDPLANE,
                        num_engines: 1,
                        enginetype: EngineType::TURBOPROP,
                        manufacturer: "CESSNA".to_string(),
                        name: "208 Caravan 1, (Super)Cargomaster, Grand  Caravan (C98, U27)"
                            .to_string(),
                    }
                ),
                (
                    "EC45".to_string(),
                    Aircraft {
                        designator: "EC45".to_string(),
                        wtc: Wtc::LIGHT,
                        aircrafttype: AircraftType::HELICOPTER,
                        num_engines: 2,
                        enginetype: EngineType::TURBOPROP,
                        manufacturer: "EUROCOPTER".to_string(),
                        name: "EC145".to_string(),
                    }
                ),
                (
                    "F260".to_string(),
                    Aircraft {
                        designator: "F260".to_string(),
                        wtc: Wtc::LIGHT,
                        aircrafttype: AircraftType::LANDPLANE,
                        num_engines: 1,
                        enginetype: EngineType::PISTON,
                        manufacturer: "AERMACCHI".to_string(),
                        name: "SF260E/F, Warrior".to_string(),
                    }
                ),
                (
                    "C08T".to_string(),
                    Aircraft {
                        designator: "C08T".to_string(),
                        wtc: Wtc::LIGHT,
                        aircrafttype: AircraftType::LANDPLANE,
                        num_engines: 0,
                        enginetype: EngineType::TURBOPROP,
                        manufacturer: "SOLOY".to_string(),
                        name: "208 DualPac Caravan, Pathfinder 21".to_string(),
                    }
                ),
                (
                    "ZZZZ".to_string(),
                    Aircraft {
                        designator: "ZZZZ".to_string(),
                        wtc: Wtc::UNKNOWN,
                        aircrafttype: AircraftType::UNKNOWN,
                        num_engines: 0,
                        enginetype: EngineType::UNKNOWN,
                        manufacturer: "-".to_string(),
                        name: "Aircraft type not assigned".to_string(),
                    }
                ),
            ])
        );
    }
}
