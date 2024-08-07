pub mod map;
pub mod settings;
pub mod symbol;

use std::collections::HashMap;
use std::io;
use std::path::PathBuf;

use pest::iterators::Pair;
use pest_derive::Parser;

use phf::phf_map;
use serde::Serialize;
use symbol::SymbolDef;
use thiserror::Error;

use crate::read_to_string;

use self::map::{parse_topsky_maps, ColourDef, LineStyleDef, MapDef, OverrideSct};
use self::settings::{parse_topsky_settings, Settings};
use self::symbol::parse_topsky_symbols;

#[derive(Error, Debug)]
pub enum TopskyError {
    #[error("failed to read topsky files: {0}")]
    FileRead(#[from] io::Error),
    #[error("failed to parse topsky maps/symbol file: {0}")]
    Parse(#[from] pest::error::Error<Rule>),
    #[error("failed to parse topsky settings file: {0}")]
    ParseSettings(#[from] pest::error::Error<settings::Rule>),
}

#[derive(Parser)]
#[grammar = "pest/base.pest"]
#[grammar = "pest/symbol_rule.pest"]
#[grammar = "pest/topsky.pest"]
pub struct TopskyParser;

type ColourMap = phf::Map<&'static str, (Option<(u8, u8, u8)>, Option<(u8, u8, u8)>)>;
pub static DEFAULT_COLOURS: ColourMap = phf_map! {
    "ACF_Via_CFL" => (Some((82,190,115)), Some((82,190,115))),
    "Active_Map" => (Some((70,90,135)), Some((198,174,58))),
    "Active_Map_Type_1" => (Some((1,1,1)), Some((87,87,164))),
    "Active_Map_Type_2" => (Some((90,90,90)), Some((80,162,74))),
    "Active_Map_Type_3" => (Some((220,205,121)), Some((179,180,180))),
    "Active_Map_Type_4" => (Some((121,66,133)), Some((200,53,18))),
    "Active_Map_Type_5" => (Some((51,102,152)), Some((198,174,58))),
    "Active_Map_Type_6" => (Some((114,69,58)), Some((114,69,58))),
    "Active_Map_Type_7" => (Some((138,20,12)), Some((138,20,12))),
    "Active_Map_Type_8" => (Some((195,186,69)), Some((195,186,69))),
    "Active_Map_Type_9" => (Some((220,125,25)), Some((220,125,25))),
    "Active_Map_Type_10" => (Some((255,255,255)), Some((255,255,255))),
    "Active_Map_Type_11" => (Some((51,134,49)), Some((51,134,49))),
    "Active_Map_Type_12" => (Some((41,102,255)), Some((41,102,255))),
    "Active_Map_Type_13" => (Some((100,160,100)), Some((100,160,100))),
    "Active_Map_Type_14" => (Some((141,184,236)), Some((141,184,236))),
    "Active_Map_Type_15" => (Some((227,213,29)), Some((227,213,29))),
    "Active_Map_Type_16" => (Some((60,60,60)), Some((60,60,60))),
    "Active_Map_Type_17" => (Some((155,155,155)), Some((155,155,155))),
    "Active_Map_Type_18" => (Some((183,26,19)), Some((183,26,19))),
    "Active_Map_Type_19" => (Some((120,91,65)), Some((120,91,65))),
    "Active_Map_Type_20" => (Some((138,69,58)), Some((138,69,58))),
    "Active_RD_Infill_Map" => (Some((165,160,160)), Some((90,90,90))),
    "Active_RD_Map" => (Some((124,20,13)), Some((150,41,43))),
    "Active_Sector" => (Some((153,154,149)), Some((52,58,62))),
    "Active_Text_Map" => (Some((210,211,211)), Some((190,190,190))),
    "AIW_Intrusion" => (Some((255,152,0)), Some((255,152,0))),
    "Arm" => (Some((97,97,97)), Some((97,97,97))),
    "Assumed" => (Some((1,0,1)), Some((220,220,220))),
    "Background" => (Some((162,163,156)), Some((74,80,85))),
    "Border" => (Some((51,51,52)), Some((51,51,52))),
    "BottomShadow" => (Some((66,66,66)), Some((70,70,70))),
    "CARD_Mark_All" => (None, Some((255,125,125))),
    "CARD_Mark_Own" => (None, Some((255,124,125))),
    "CARD_Min_Sep" => (Some((209,207,211)), Some((61,61,61))),
    "CARD_Reminder" => (None, Some((170,231,198))),
    "CARD_Symbol_Fg" => (Some((0,1,0)), Some((9,10,11))),
    "CARD_Time_Vector" => (Some((95,95,95)), Some((171,231,197))),
    "COL_Above_Threshold" => (None, Some((239,225,41))),
    "COL_Under_Threshold" => (None, Some((220,220,221))),
    "Concerned" => (Some((124,1,124)), Some((111,153,110))),
    "Conflict_Ack" => (Some((110,98,98)), Some((135,127,118))),
    "Conflict_Ack_FL" => (Some((110,98,98)), Some((135,127,117))),
    "Coordination" => (Some((0,0,185)), Some((150,215,150))),
    "CPDLC_Controller_Late" => (Some((235,225,108)), Some((170,78,39))),
    "CPDLC_Discarded" => (Some((141,141,141)), Some((128,128,128))),
    "CPDLC_DM_Request" => (Some((205,252,254)), Some((30,250,250))),
    "CPDLC_Failed" => (Some((169,8,9)), Some((170,78,39))),
    "CPDLC_Pilot_Late" => (Some((246,164,96)), Some((170,78,40))),
    "CPDLC_Standby" => (Some((170,248,87)), Some((170,78,39))),
    "CPDLC_UM_Clearance" => (Some((2,2,2)), Some((30,251,250))),
    "CPDLC_Unable" => (Some((247,164,96)), Some((170,77,39))),
    "CPDLC_Urgency" => (Some((237,229,108)), Some((226,25,25))),
    "Datalink_Logged_On" => (Some((120,120,120)), Some((145,145,145))),
    "Deselected" => (Some((127,127,127)), Some((95,95,96))),
    "East_NAT_Map" => (Some((180,180,1)), Some((180,180,1))),
    "Field_Highlight" => (Some((209,207,211)), Some((72,72,73))),
    "Flight_Highlight" => (Some((190,190,185)), Some((36,41,45))),
    "Flight_Leg" => (Some((205,252,255)), Some((170,231,197))),
    "Foreground" => (Some((0,1,0)), Some((200,200,200))),
    "FPLSEP_Tool_1" => (Some((255,170,46)), None),
    "FPLSEP_Tool_2" => (Some((204,122,0)), None),
    "FPLSEP_Tool_3" => (Some((255,195,75)), None),
    "FPLSEP_Tool_4" => (Some((223,143,1)), None),
    "FPLSEP_Tool_5" => (Some((170,102,0)), None),
    "Freq_Indicator" => (Some((209,207,211)), Some((114,136,255))),
    "Global_Menu_Highlight" => (Some((211,211,211)), Some((1,165,219))),
    "Heading_Vector" => (Some((200,200,200)), Some((170,232,197))),
    "Info_Coord" => (Some((244,164,96)), Some((1,255,255))),
    "Information" => (Some((169,249,86)), Some((40,210,40))),
    "Information_FL" => (Some((0,200,1)), Some((41,245,41))),
    "Informed" => (Some((25,109,25)), Some((175,125,175))),
    "Informed_2" => (Some((25,109,25)), Some((104,163,195))),
    "Informed_3" => (Some((25,109,25)), Some((147,124,108))),
    "LatLong_Info" => (None, Some((169,231,197))),
    "Map_1" => (Some((121,66,133)), Some((80,95,95))),
    "Map_2" => (Some((115,115,112)), Some((44,44,44))),
    "Map_3" => (Some((213,241,155)), Some((100,108,111))),
    "Map_4" => (Some((164,164,157)), Some((52,58,62))),
    "Map_Auto_Label" => (Some((115,115,112)), Some((159,159,160))),
    "Map_Auto_Symbol" => (Some((115,115,112)), Some((159,159,160))),
    "Map_Border" => (Some((117,117,117)), Some((134,134,135))),
    "Map_Hotspot" => (Some((72,72,70)), Some((87,139,200))),
    "Map_Info" => (Some((217,217,216)), Some((150,150,150))),
    "Map_Land" => (Some((133,133,138)), Some((140,140,0))),
    "Map_Symbol" => (Some((55,55,55)), Some((110,130,140))),
    "MQDM" => (None, Some((155,140,115))),
    "Negotiation_In" => (None, Some((220,125,175))),
    "Negotiation_Out" => (None, Some((220,125,175))),
    "Normal_Load" => (Some((1,50,1)), Some((75,75,78))),
    "Oceanic_Level_Highlight" => (Some((240,225,42)), Some((240,225,41))),
    "Overflown" => (Some((169,249,86)), Some((82,105,146))),
    "Overload" => (Some((255,128,1)), Some((224,26,25))),
    "Potential" => (Some((41,40,216)), Some((41,40,216))),
    "Potential_FL" => (Some((40,41,216)), Some((40,41,216))),
    "Preactive_Map" => (Some((116,115,112)), Some((154,138,128))),
    "Preactive_Text_Map" => (Some((210,211,212)), Some((155,147,138))),
    "Predisplay_Map" => (Some((190,190,190)), Some((175,130,130))),
    "Proposition_Accepted" => (Some((255,255,0)), Some((255,255,0))),
    "Proposition_In" => (Some((254,255,255)), Some((220,125,175))),
    "Proposition_Out" => (Some((254,255,255)), Some((220,125,175))),
    "QDM" => (Some((200,200,200)), Some((215,190,163))),
    "Radar_Win_Bg" => (Some((164,164,157)), Some((67,73,76))),
    "Raw_Video1" => (Some((55,85,115)), Some((55,85,115))),
    "Raw_Video2" => (Some((50,80,110)), Some((50,80,110))),
    "Raw_Video3" => (Some((45,75,105)), Some((45,75,105))),
    "Raw_Video4" => (Some((40,70,100)), Some((40,70,100))),
    "Raw_Video5" => (Some((35,65,95)), Some((35,65,95))),
    "Raw_Video6" => (Some((30,60,90)), Some((30,60,90))),
    "Raw_Video7" => (Some((25,55,85)), Some((25,55,85))),
    "Redundant" => (Some((156,96,59)), Some((185,140,89))),
    "Reminder" => (None, Some((165,145,225))),
    "Runway" => (Some((200,200,160)), Some((116,200,71))),
    "Rwy_App_Line_Inuse" => (Some((220,220,220)), Some((82,190,115))),
    "Rwy_App_Line_Not_Inuse" => (Some((220,205,121)), Some((135,135,70))),
    "Rwy_Locked" => (Some((124,1,124)), Some((41,210,41))),
    "Select" => (Some((97,97,97)), Some((151,215,150))),
    "Selected" => (Some((210,210,210)), Some((61,121,148))),
    "Selected_Group" => (Some((226,210,210)), None),
    "Selected_Period" => (Some((220,40,70)), Some((255,255,41))),
    "SEP_Tool_1" => (Some((205,252,255)), Some((153,217,234))),
    "SEP_Tool_2" => (Some((25,210,230)), Some((255,153,184))),
    "SEP_Tool_3" => (Some((120,245,250)), Some((255,209,143))),
    "SEP_Tool_4" => (Some((185,240,244)), Some((197,64,212))),
    "SEP_Tool_5" => (Some((25,180,210)), Some((140,140,255))),
    "SEP_Tool_6" => (None, Some((95,170,140))),
    "SEP_Tool_7" => (None, Some((185,130,85))),
    "SEP_Vert" => (None, Some((160,150,135))),
    "Sid_Star_Allocation" => (Some((124,1,124)), Some((15,185,15))),
    "SMW_Highlight" => (Some((255,255,255)), Some((253,255,255))),
    "SMW_Level_Band" => (Some((169,249,86)), Some((24,209,114))),
    "SMW_Overflight" => (Some((236,228,108)), Some((254,152,1))),
    "SMW_Overlap" => (Some((168,7,8)), Some((224,25,25))),
    "SMW_Overlap_Box" => (Some((0,1,0)), Some((207,207,207))),
    "SMW_Overshoot" => (Some((236,228,108)), Some((239,224,40))),
    "Standard_Line_RDF" => (Some((91,134,76)), Some((91,134,76))),
    "Standard_RDF" => (Some((11,12,19)), Some((11,12,19))),
    "Suite_Highlight" => (Some((236,228,108)), Some((0,220,255))),
    "System_Calculated_TOC" => (None, Some((170,230,197))),
    "System_Calculated_TOD" => (None, Some((170,231,197))),
    "Temp_Track_Highlight" => (Some((0,164,220)), Some((0,164,220))),
    "Text_Notes" => (Some((0,0,0)), Some((255,0,0))),
    "TopShadow" => (Some((130,130,130)), Some((155,158,159))),
    "Track_Default" => (Some((188,188,188)), Some((210,210,210))),
    "Track_Highlight" => (Some((255,255,255)), Some((0,165,219))),
    "Trough" => (Some((97,97,97)), Some((97,99,97))),
    "TSA_Active" => (Some((220,205,120)), Some((93,138,195))),
    "TSA_Border_Highlight" => (Some((255,254,255)), Some((255,254,255))),
    "TSA_Filter" => (Some((255,253,255)), None),
    "TSA_Preactive" => (Some((80,80,80)), Some((132,142,139))),
    "Unconcerned" => (Some((110,98,98)), Some((135,128,118))),
    "Unknown" => (Some((237,228,108)), Some((239,224,42))),
    "Urgency" => (Some((236,32,0)), Some((225,25,26))),
    "Urgency_FL" => (Some((166,11,1)), Some((225,25,25))),
    "VAW_Profile" => (Some((130,204,240)), Some((152,202,172))),
    "VAW_Sector_Limits" => (Some((190,190,185)), Some((145,95,30))),
    "VAW_Track_Position" => (Some((190,190,185)), Some((219,219,219))),
    "VFR" => (Some((110,1,10)), Some((110,1,10))),
    "Warning" => (Some((236,228,108)), Some((240,225,41))),
    "Warning_FL" => (Some((235,228,108)), Some((240,226,41))),
    "Weather_Map" => (Some((99,99,98)), Some((0,0,86))),
    "West_NAT_Map" => (Some((40,140,255)), Some((40,140,255))),
    "WM_Active_Fg" => (Some((230,230,231)), Some((255,254,254))),
    "WM_Bg" => (Some((147,147,145)), Some((100,100,105))),
    "WM_Border" => (Some((50,50,50)), Some((88,95,99))),
    "WM_Fg" => (Some((1,1,1)), Some((180,184,181))),
    "WM_Frame" => (Some((1,1,0)), Some((88,95,99))),
};

#[derive(Clone, Debug, Serialize)]
pub struct Topsky {
    pub symbols: HashMap<String, SymbolDef>,
    pub maps: Vec<MapDef>,
    pub colours: HashMap<String, ColourDef>,
    pub settings: Settings,
    pub line_styles: HashMap<String, LineStyleDef>,
    pub overrides: Vec<OverrideSct>,
}

fn parse_point(pair: Pair<Rule>) -> (f64, f64) {
    let mut point = pair.into_inner();
    let x = point.next().unwrap().as_str().parse().unwrap();
    let y = point.next().unwrap().as_str().parse().unwrap();
    (x, y)
}

pub type TopskyResult = Result<Topsky, TopskyError>;
impl Topsky {
    pub fn parse(path: PathBuf) -> TopskyResult {
        let (mut colours, settings) = parse_topsky_settings(&read_to_string(&fs_err::read(
            path.join("TopSkySettings.txt"),
        )?)?)?;
        let mut symbols = fs_err::read(path.join("TopSkySymbols.txt"))
            .map_or_else(|_| Ok(HashMap::new()), |bytes| parse_topsky_symbols(&bytes))?;
        let (maps, mapsymbols, mapcolours, line_styles, overrides) =
            parse_topsky_maps(&fs_err::read(path.join("TopSkyMaps.txt"))?)?;
        symbols.extend(mapsymbols);
        colours.extend(mapcolours);

        Ok(Topsky {
            symbols,
            maps,
            colours,
            settings,
            line_styles,
            overrides,
        })
    }
}
