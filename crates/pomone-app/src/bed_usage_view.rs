//! Home-page bed-usage curve (issue #51, phase B).
//!
//! Produces a 52-point weekly series describing how much of the farm's bed
//! area is occupied by an annual crop, with a parallel series restricted to
//! **sheltered** beds (under cover). The home page draws the two as curves so
//! the grower sees open-field vs greenhouse utilisation at a glance.
//!
//! ## Definitions (documented so they're easy to challenge)
//!
//! * **Bed** = a *leaf* location (one with no child locations) that does **not**
//!   host a perennial planting. Leaves are the actual growing sub-divisions
//!   (planches, greenhouse beds); excluding perennial-bearing leaves drops
//!   orchard rows and hedges, which aren't annual beds. Empty leaves still
//!   count — an unused bed should show as under-utilised, not vanish.
//! * **Occupied in week _w_** = the bed carries an annual `Cycle` planting of
//!   the **current season** (its first harvest falls in `season_year`, the same
//!   filter the home Gantt uses) whose occupancy window —
//!   `min(sown, transplanted, first_harvest)` → `last_harvest`, clamped to the
//!   season year — covers any day of week _w_ (7-day buckets of the year).
//!   Aligning the filter + clamp with the Gantt keeps the curve consistent
//!   with the bars above it.
//! * **Sheltered** = the bed's own kind is `covered`, or any ancestor
//!   location's kind is (a planche inside a Serre is sheltered).
//! * **Two disjoint groups** — sheltered beds and the *other* (open-field)
//!   beds. Each curve is that group's own occupancy: occupied area ÷ that
//!   group's total area, as a percentage (0 when the group is empty). So a
//!   farm reads them as "greenhouses are 100% full, open field is 40% full",
//!   not one nested inside the other.
//!
//! Perennials are out of scope (they'd span the whole axis and swamp the
//! seasonal signal) — see issue #51.

use crate::error::AppResult;
use chrono::{Datelike, NaiveDate};
use pomone_db::Repository;
use pomone_domain::{Location, LocationId, LocationKindId, PlantingSchedule};
use rust_decimal::prelude::ToPrimitive;
use std::collections::{HashMap, HashSet};

/// The bed-usage curve plus presence flags. The flags let the UI tell "no
/// beds at all" (empty state) from "beds present but unoccupied" (flat 0%
/// curve), and hide a group's curve when that group has no beds.
#[derive(Debug, Clone, PartialEq)]
pub struct BedUsage {
    /// 52 weekly points, week 1 first.
    pub points: Vec<BedUsagePoint>,
    /// Whether the farm has any open-field (non-sheltered) bed.
    pub has_open_beds: bool,
    /// Whether the farm has any sheltered bed.
    pub has_sheltered_beds: bool,
}

/// One week of the bed-usage curve. The two percentages cover **disjoint**
/// groups of beds, each normalised within its own group.
#[derive(Debug, Clone, PartialEq)]
pub struct BedUsagePoint {
    /// 1..=52 (7-day buckets of the season year; the last week absorbs the
    /// 1–2 trailing days).
    pub week: u32,
    /// Percent of the **open-field** (non-sheltered) bed area occupied that
    /// week (0.0..=100.0). `0.0` when there are no open-field beds.
    pub open_pct: f64,
    /// Percent of the **sheltered** bed area occupied that week. `0.0` when
    /// there are no sheltered beds.
    pub sheltered_pct: f64,
}

/// Number of weekly buckets in the series.
const WEEKS: u32 = 52;

/// Internal per-bed accumulator.
struct Bed {
    area: f64,
    sheltered: bool,
    occupied_weeks: HashSet<u32>,
}

/// Build the 52-week bed-usage series (week 1 first) with presence flags for
/// `season_year` — the same season the home Gantt shows.
pub async fn bed_usage_series(repo: &dyn Repository, season_year: i32) -> AppResult<BedUsage> {
    let locations = repo.location_list().await?;
    let kinds = repo.location_kind_list().await?;
    let plantings = repo.planting_list().await?;

    let covered_kind: HashMap<LocationKindId, bool> =
        kinds.iter().map(|k| (k.id, k.covered)).collect();
    let loc_by_id: HashMap<LocationId, &Location> = locations.iter().map(|l| (l.id, l)).collect();

    // A location is a leaf when nothing else lists it as a parent.
    let parents: HashSet<LocationId> = locations.iter().filter_map(|l| l.parent_id).collect();
    // Leaves bearing a perennial are orchard rows / hedges, not annual beds.
    let perennial_locs: HashSet<LocationId> = plantings
        .iter()
        .filter(|p| matches!(p.schedule, PlantingSchedule::Perennial { .. }))
        .map(|p| p.location_id)
        .collect();

    // One accumulator per bed.
    let mut beds: HashMap<LocationId, Bed> = locations
        .iter()
        .filter(|l| !parents.contains(&l.id) && !perennial_locs.contains(&l.id))
        .map(|l| {
            let area = (l.length_m * l.width_m).to_f64().unwrap_or(0.0);
            let sheltered = is_sheltered(l.id, &loc_by_id, &covered_kind);
            (
                l.id,
                Bed {
                    area,
                    sheltered,
                    occupied_weeks: HashSet::new(),
                },
            )
        })
        .collect();

    // Fold each in-season annual planting's occupancy months onto its bed.
    // Same filter (first harvest in `season_year`) and clamp the Gantt uses.
    let season_start = NaiveDate::from_ymd_opt(season_year, 1, 1).unwrap_or_default();
    let season_end = NaiveDate::from_ymd_opt(season_year, 12, 31).unwrap_or_default();
    for p in &plantings {
        if let PlantingSchedule::Cycle {
            sown_on,
            transplanted_on,
            first_harvest_on,
            last_harvest_on,
        } = p.schedule
        {
            if first_harvest_on.year() != season_year {
                continue;
            }
            let Some(bed) = beds.get_mut(&p.location_id) else {
                continue; // planting on a non-bed location — ignore.
            };
            let raw_start = [sown_on, transplanted_on, Some(first_harvest_on)]
                .into_iter()
                .flatten()
                .min()
                .unwrap_or(first_harvest_on);
            // Clamp the window to the season year (winter-sow before Jan, or a
            // harvest spilling past Dec) so weeks stay within 1..=52.
            let start = raw_start.max(season_start);
            let end = last_harvest_on.min(season_end);
            for w in week_of_doy(start.ordinal())..=week_of_doy(end.ordinal()) {
                bed.occupied_weeks.insert(w);
            }
        }
    }

    // Two disjoint denominators: open-field beds and sheltered beds.
    let total_open: f64 = beds.values().filter(|b| !b.sheltered).map(|b| b.area).sum();
    let total_sheltered: f64 = beds.values().filter(|b| b.sheltered).map(|b| b.area).sum();

    let points = (1..=WEEKS)
        .map(|week| {
            let occ_open: f64 = beds
                .values()
                .filter(|b| !b.sheltered && b.occupied_weeks.contains(&week))
                .map(|b| b.area)
                .sum();
            let occ_sheltered: f64 = beds
                .values()
                .filter(|b| b.sheltered && b.occupied_weeks.contains(&week))
                .map(|b| b.area)
                .sum();
            BedUsagePoint {
                week,
                open_pct: pct(occ_open, total_open),
                sheltered_pct: pct(occ_sheltered, total_sheltered),
            }
        })
        .collect();

    Ok(BedUsage {
        points,
        has_open_beds: total_open > 0.0,
        has_sheltered_beds: total_sheltered > 0.0,
    })
}

/// `numerator / denominator * 100`, guarding the empty-farm case.
fn pct(numerator: f64, denominator: f64) -> f64 {
    if denominator <= 0.0 {
        0.0
    } else {
        numerator / denominator * 100.0
    }
}

/// True when `start` or any ancestor location has a covered kind. Bounded by a
/// depth guard so a malformed parent cycle can't loop forever.
fn is_sheltered(
    start: LocationId,
    loc_by_id: &HashMap<LocationId, &Location>,
    covered_kind: &HashMap<LocationKindId, bool>,
) -> bool {
    let mut cur = Some(start);
    let mut guard = 0;
    while let Some(id) = cur {
        if guard > 64 {
            break;
        }
        guard += 1;
        let Some(loc) = loc_by_id.get(&id) else { break };
        if *covered_kind.get(&loc.kind_id).unwrap_or(&false) {
            return true;
        }
        cur = loc.parent_id;
    }
    false
}

/// Week bucket (1..=[`WEEKS`]) for a 1-based day-of-year. Buckets are 7 days
/// wide; the last bucket absorbs the 1–2 trailing days of the year so the
/// series stays a fixed 52 points.
fn week_of_doy(doy: u32) -> u32 {
    (((doy.saturating_sub(1)) / 7) + 1).min(WEEKS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::seed_test_data;
    use pomone_db::{
        seed_defaults, LocationKindRepo, LocationRepo, PlantingRepo, SqliteRepository, VarietyRepo,
    };
    use pomone_domain::{Lifespan, Planting, VarietyId};
    use rust_decimal_macros::dec;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    async fn repo() -> SqliteRepository {
        let r = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&r).await.unwrap();
        r
    }

    /// Repo with a valid crop + variety (so plantings satisfy their FK).
    async fn repo_with_variety() -> (SqliteRepository, VarietyId) {
        let r = repo().await;
        seed_test_data(&r).await.unwrap();
        let v = r.variety_list().await.unwrap().into_iter().next().unwrap();
        (r, v.id)
    }

    #[test]
    fn week_of_doy_buckets_and_caps() {
        assert_eq!(week_of_doy(1), 1);
        assert_eq!(week_of_doy(7), 1);
        assert_eq!(week_of_doy(8), 2);
        // Day 365/366 fall in the capped final bucket, not a 53rd.
        assert_eq!(week_of_doy(365), WEEKS);
        assert_eq!(week_of_doy(366), WEEKS);
    }

    #[tokio::test]
    async fn empty_farm_is_all_zero() {
        let r = repo().await;
        let u = bed_usage_series(&r, 2026).await.unwrap();
        assert_eq!(u.points.len(), WEEKS as usize);
        assert!(!u.has_open_beds && !u.has_sheltered_beds);
        assert!(u
            .points
            .iter()
            .all(|p| p.open_pct == 0.0 && p.sheltered_pct == 0.0));
    }

    /// A bed needs a kind; grab a seeded one by name.
    async fn kind_id(r: &SqliteRepository, name: &str) -> LocationKindId {
        r.location_kind_list()
            .await
            .unwrap()
            .into_iter()
            .find(|k| k.name == name)
            .unwrap()
            .id
    }

    async fn add_annual(
        r: &SqliteRepository,
        variety: VarietyId,
        loc: LocationId,
        first: NaiveDate,
        last: NaiveDate,
    ) {
        let p = Planting::new(
            variety,
            loc,
            Lifespan::Annual,
            dec!(1),
            1,
            PlantingSchedule::cycle(Some(first), None, first, last).unwrap(),
            None,
            None,
        )
        .unwrap();
        r.planting_create(&p).await.unwrap();
    }

    #[tokio::test]
    async fn occupancy_splits_open_field_and_sheltered() {
        // seed_test_data adds one open leaf bed "Planche A" (25 × 0.8 = 20 m²,
        // empty). We add a 10 m² open bed and a 10 m² sheltered bed.
        // Open-field group = Planche A + open = 30 m²; sheltered group = 10 m².
        let (r, variety) = repo_with_variety().await;
        let planche = kind_id(&r, "Planche").await;
        let serre = kind_id(&r, "Serre").await; // covered

        let open = Location::new(planche, "Planche B", dec!(10), dec!(1), None, None).unwrap();
        r.location_create(&open).await.unwrap();
        let tunnel = Location::new(serre, "Tunnel 1", dec!(20), dec!(5), None, None).unwrap();
        r.location_create(&tunnel).await.unwrap();
        let inside = Location::new(
            planche,
            "Planche S1",
            dec!(10),
            dec!(1),
            Some(tunnel.id),
            None,
        )
        .unwrap();
        r.location_create(&inside).await.unwrap();

        // Open bed occupied May–July; sheltered bed occupied Feb–March.
        add_annual(&r, variety, open.id, d(2026, 5, 1), d(2026, 7, 31)).await;
        add_annual(&r, variety, inside.id, d(2026, 2, 1), d(2026, 3, 31)).await;

        let u = bed_usage_series(&r, 2026).await.unwrap();
        assert!(u.has_open_beds && u.has_sheltered_beds);
        let s = u.points;
        let week_for = |m: u32, day: u32| week_of_doy(d(2026, m, day).ordinal());
        let at = |w: u32| s.iter().find(|p| p.week == w).unwrap();

        // Mid-June: the 10 m² open bed is occupied out of the 30 m² open-field
        // group → 33.3%; sheltered group untouched → 0%.
        assert!((at(week_for(6, 15)).open_pct - 100.0 / 3.0).abs() < 1e-6);
        assert!((at(week_for(6, 15)).sheltered_pct - 0.0).abs() < 1e-6);
        // Mid-February: only the sheltered bed is occupied → open-field 0%,
        // sheltered 100% (it is the whole sheltered group).
        assert!((at(week_for(2, 15)).open_pct - 0.0).abs() < 1e-6);
        assert!((at(week_for(2, 15)).sheltered_pct - 100.0).abs() < 1e-6);
        // Mid-September: nothing growing.
        assert!((at(week_for(9, 15)).open_pct - 0.0).abs() < 1e-6);
    }
}
