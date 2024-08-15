use serde::Serialize;

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
pub struct Airline {
    pub designator: String,
    pub airline: String,
    pub callsign: String,
    pub country: String,
}
