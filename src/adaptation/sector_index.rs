use std::collections::HashMap;
use std::sync::OnceLock;

use geo::{BoundingRect as _, Contains as _, Line, Point};
use rstar::{
    primitives::{GeomWithData, Rectangle},
    RTree, AABB,
};

use super::sectors::{Sectors, Volume};

type Entry = GeomWithData<Rectangle<[f64; 2]>, (String, Volume)>;

fn rectangle_for(volume: &Volume) -> Option<Rectangle<[f64; 2]>> {
    let rect = volume.lateral_border.bounding_rect()?;
    let min = rect.min();
    let max = rect.max();
    Some(Rectangle::from_corners([min.x, min.y], [max.x, max.y]))
}

fn build_tree(sectors: &Sectors, volumes: &HashMap<String, Volume>) -> RTree<Entry> {
    let entries: Vec<Entry> = sectors
        .iter()
        .filter(|(_, sector)| !sector.position_priority.is_empty())
        .flat_map(|(_, sector)| {
            sector
                .volumes
                .iter()
                .filter_map(|vol_id| volumes.get(vol_id))
                .map(move |volume| (sector.id.clone(), volume.clone()))
        })
        .filter_map(|(sector_id, volume)| {
            let rectangle = rectangle_for(&volume)?;
            Some(GeomWithData::new(rectangle, (sector_id, volume)))
        })
        .collect();
    RTree::bulk_load(entries)
}

/// Spatial index over sector volumes, keyed by each volume's lateral bounding box.
///
/// Only sectors with a non-empty `position_priority` are indexed.
///
/// If `sectors`/`volumes` are mutated *after* the index has already been queried once,
/// the cached tree goes stale, call `Adaptation::rebuild_sector_index` explicitly after
/// such a mutation.
#[derive(Debug, Default)]
pub struct SectorVolumeIndex(OnceLock<RTree<Entry>>);

impl Clone for SectorVolumeIndex {
    fn clone(&self) -> Self {
        let cell = OnceLock::new();
        if let Some(tree) = self.0.get() {
            cell.set(tree.clone()).ok();
        }
        Self(cell)
    }
}

impl SectorVolumeIndex {
    fn get_or_build<'a>(
        &'a self,
        sectors: &Sectors,
        volumes: &HashMap<String, Volume>,
    ) -> &'a RTree<Entry> {
        self.0.get_or_init(|| build_tree(sectors, volumes))
    }

    /// Forces a rebuild, overwriting any already-cached tree.
    pub(crate) fn rebuild(&mut self, sectors: &Sectors, volumes: &HashMap<String, Volume>) {
        self.0 = OnceLock::new();
        self.0.set(build_tree(sectors, volumes)).ok();
    }

    /// All volumes whose lateral border contains `coordinate`, regardless of level --
    /// callers needing a level-range filter (e.g. `vertical_border_wpts`, which scans across
    /// levels to find a boundary) apply that themselves.
    pub fn volumes_at<'a>(
        &'a self,
        sectors: &Sectors,
        volumes: &HashMap<String, Volume>,
        coordinate: Point,
    ) -> impl Iterator<Item = &'a (String, Volume)> {
        self.get_or_build(sectors, volumes)
            .locate_all_at_point([coordinate.x(), coordinate.y()])
            .map(|entry| &entry.data)
            .filter(move |(_, volume)| volume.lateral_border.contains(&coordinate))
    }

    /// Coarse candidates whose bounding box laterally intersects `line`'s bounding box,
    /// callers still need their own exact `intersects`/`line_string_intersection` check
    /// on the returned candidates.
    pub fn volumes_near_line<'a>(
        &'a self,
        sectors: &Sectors,
        volumes: &HashMap<String, Volume>,
        line: Line,
    ) -> impl Iterator<Item = &'a (String, Volume)> {
        let rect = line.bounding_rect();
        let envelope =
            AABB::from_corners([rect.min().x, rect.min().y], [rect.max().x, rect.max().y]);
        self.get_or_build(sectors, volumes)
            .locate_in_envelope_intersecting(envelope)
            .map(|entry| &entry.data)
    }

    /// Level-aware lookup of the sector containing `coordinate` at `level_ft`.
    pub fn find_sector(
        &self,
        sectors: &Sectors,
        volumes: &HashMap<String, Volume>,
        coordinate: Point,
        // TODO uom?
        level_ft: f32,
    ) -> Option<&str> {
        self.volumes_at(sectors, volumes, coordinate)
            .filter(|(_, volume)| {
                level_ft >= volume.lower_level as f32 && level_ft < volume.upper_level as f32
            })
            // stable return value in case of overlapping data
            .min_by_key(|(id, _)| id.as_str())
            .map(|(id, _)| id.as_str())
    }
}

#[cfg(test)]
mod tests {
    use geo::{line_string, point, Line};

    use super::*;
    use crate::adaptation::sectors::Sector;

    fn square_volume(id: &str, min: (f64, f64), max: (f64, f64), lower: u32, upper: u32) -> Volume {
        let border = line_string![
            (x: min.0, y: min.1),
            (x: max.0, y: min.1),
            (x: max.0, y: max.1),
            (x: min.0, y: max.1),
            (x: min.0, y: min.1),
        ];
        Volume::new(id.to_string(), lower, upper, border)
    }

    fn sector(id: &str, volumes: &[&str]) -> Sector {
        Sector {
            id: id.to_string(),
            position_priority: vec!["POS".to_string()],
            runway_filter: vec![],
            volumes: volumes.iter().map(|v| (*v).to_string()).collect(),
            departure_aerodromes: std::collections::HashSet::default(),
            arrival_aerodromes: std::collections::HashSet::default(),
        }
    }

    fn built(
        entries: Vec<(Sector, Volume)>,
    ) -> (Sectors, HashMap<String, Volume>, SectorVolumeIndex) {
        let mut sectors = HashMap::new();
        let mut volumes = HashMap::new();
        for (sector, volume) in entries {
            volumes.insert(volume.id.clone(), volume);
            sectors.insert(sector.id.clone(), sector);
        }
        (Sectors(sectors), volumes, SectorVolumeIndex::default())
    }

    #[test]
    fn point_inside_volume_returns_sector() {
        let vol = square_volume("VOL1", (0.0, 0.0), (10.0, 10.0), 0, 20_000);
        let (sectors, volumes, index) = built(vec![(sector("SEC1", &["VOL1"]), vol)]);

        assert_eq!(
            index.find_sector(&sectors, &volumes, point! { x: 5.0, y: 5.0 }, 5000.0),
            Some("SEC1")
        );
    }

    #[test]
    fn point_outside_volume_returns_none() {
        let vol = square_volume("VOL1", (0.0, 0.0), (10.0, 10.0), 0, 20_000);
        let (sectors, volumes, index) = built(vec![(sector("SEC1", &["VOL1"]), vol)]);

        assert_eq!(
            index.find_sector(&sectors, &volumes, point! { x: 50.0, y: 50.0 }, 5000.0),
            None
        );
    }

    #[test]
    fn level_outside_range_returns_none() {
        let vol = square_volume("VOL1", (0.0, 0.0), (10.0, 10.0), 10_000, 20_000);
        let (sectors, volumes, index) = built(vec![(sector("SEC1", &["VOL1"]), vol)]);

        assert_eq!(
            index.find_sector(&sectors, &volumes, point! { x: 5.0, y: 5.0 }, 5000.0),
            None
        );
        assert_eq!(
            index.find_sector(&sectors, &volumes, point! { x: 5.0, y: 5.0 }, 15_000.0),
            Some("SEC1")
        );
        // upper_level is exclusive
        assert_eq!(
            index.find_sector(&sectors, &volumes, point! { x: 5.0, y: 5.0 }, 20_000.0),
            None
        );
    }

    #[test]
    fn sector_without_position_priority_is_not_indexed() {
        let vol = square_volume("VOL1", (0.0, 0.0), (10.0, 10.0), 0, 20_000);
        let mut sec = sector("SEC1", &["VOL1"]);
        sec.position_priority = vec![];
        let (sectors, volumes, index) = built(vec![(sec, vol)]);

        assert_eq!(
            index.find_sector(&sectors, &volumes, point! { x: 5.0, y: 5.0 }, 5000.0),
            None
        );
    }

    #[test]
    fn volumes_near_line_finds_crossing_candidate() {
        let vol = square_volume("VOL1", (5.0, -5.0), (15.0, 5.0), 0, 20_000);
        let (sectors, volumes, index) = built(vec![(sector("SEC1", &["VOL1"]), vol)]);

        let crossing = Line::new(point! { x: 0.0, y: 0.0 }, point! { x: 20.0, y: 0.0 });
        let candidates: Vec<_> = index
            .volumes_near_line(&sectors, &volumes, crossing)
            .collect();
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].0, "SEC1");

        let far_away = Line::new(point! { x: 100.0, y: 100.0 }, point! { x: 200.0, y: 100.0 });
        assert_eq!(
            index
                .volumes_near_line(&sectors, &volumes, far_away)
                .count(),
            0
        );
    }

    #[test]
    fn rebuild_overwrites_a_stale_cached_tree() {
        let vol = square_volume("VOL1", (0.0, 0.0), (10.0, 10.0), 0, 20_000);
        let (sectors, volumes, mut index) = built(vec![(sector("SEC1", &["VOL1"]), vol)]);
        assert_eq!(
            index.find_sector(&sectors, &volumes, point! { x: 5.0, y: 5.0 }, 5000.0),
            Some("SEC1")
        );

        let vol2 = square_volume("VOL2", (0.0, 0.0), (10.0, 10.0), 0, 20_000);
        let (sectors2, volumes2, _) = built(vec![(sector("SEC2", &["VOL2"]), vol2)]);
        index.rebuild(&sectors2, &volumes2);

        assert_eq!(
            index.find_sector(&sectors2, &volumes2, point! { x: 5.0, y: 5.0 }, 5000.0),
            Some("SEC2")
        );
    }
}
