# VATSIM Parsers

This is a rust library that supports parsing and serialising a number of
different formats, some of these are currently only implemented in parts,
pull requests are welcome.

## Supported Formats

### .sct files

.sct or Sector files are the base format for providing location information
for airports, waypoints, navaids etc. This parser so far implements:

- Airports
- Runways
- Fixes
- NDBs
- VORs

### .prf files

Euroscope profile files contain information about which other settings files,
plugins and .asr files to load.

### Symbology settings

Euroscope settings for colours, symbols, font sizes, etc.

Currently only supports colour and font size.

### .asr files

Display settings for Euroscope, most items should be supported except for
sector file items.

### Topsky files

Can read various files for the Topsky Euroscope plugin, including symbols,
maps and colours
