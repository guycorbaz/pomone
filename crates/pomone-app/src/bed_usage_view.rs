//! Home-page bed-usage curve (issue #51, phase B).
//!
//! Produces a 12-point monthly series describing how much of the farm's bed
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
//! * **Occupied in month _m_** = the bed carries an annual `Cycle` planting
//!   whose occupancy window — `min(sown, transplanted, first_harvest)` →
//!   `last_harvest` — covers any day of month _m_. Windows are projected onto a
//!   single representative 12-month axis by month-of-year, mirroring the Crop
//!   Map's day-of-year model (so the year itself is ignored).
//! * **Sheltered** = the bed's own kind is `covered`, or any ancestor
//!   location's kind is (a planche inside a Serre is sheltered).
//! * **Metric** = occupied bed *area* ÷ total bed area, as a percentage. The
//!   sheltered series normalises within the sheltered subset (0 when there are
//!   no sheltered beds).
//!
//! Perennials are out of scope (they'd span the whole axis and swamp the
//! seasonal signal) — see issue #51.

use crate::error::AppResult;
use chrono::{Datelike, Months, NaiveDate};
use pomone_db::Repository;
use pomone_domain::{Location, LocationId, LocationKindId, PlantingSchedule};
use rust_decimal::prelude::ToPrimitive;
use std::collections::{HashMap, HashSet};

/// One month of the bed-usage curve.
#[derive(Debug, Clone, PartialEq)]
pub struct BedUsagePoint {
    /// 1..=12.
    pub month: u32,
    /// Percent of total bed area occupied that month (0.0..=100.0).
    pub all_pct: f64,
    /// Same, restricted to sheltered beds. `0.0` when there are none.
    pub sheltered_pct: f64,
}

/// Internal per-bed accumulator.
struct Bed {
    area: f64,
    sheltered: bool,
    occupied_months: HashSet<u32>,
}

/// Build the 12-month bed-usage series (index 0 = January).
pub async fn bed_usage_series(repo: &dyn Repository) -> AppResult<Vec<BedUsagePoint>> {
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
                    occupied_months: HashSet::new(),
                },
            )
        })
        .collect();

    // Fold each annual planting's occupancy months onto its bed.
    for p in &plantings {
        if let PlantingSchedule::Cycle {
            sown_on,
            transplanted_on,
            first_harvest_on,
            last_harvest_on,
        } = p.schedule
        {
            let Some(bed) = beds.get_mut(&p.location_id) else {
                continue; // planting on a non-bed location — ignore.
            };
            let start = [sown_on, transplanted_on, Some(first_harvest_on)]
                .into_iter()
                .flatten()
                .min()
                .unwrap_or(first_harvest_on);
            for m in months_between(start, last_harvest_on) {
                bed.occupied_months.insert(m);
            }
        }
    }

    let total_all: f64 = beds.values().map(|b| b.area).sum();
    let total_sheltered: f64 = beds.values().filter(|b| b.sheltered).map(|b| b.area).sum();

    let series = (1..=12)
        .map(|month| {
            let occ_all: f64 = beds
                .values()
                .filter(|b| b.occupied_months.contains(&month))
                .map(|b| b.area)
                .sum();
            let occ_sheltered: f64 = beds
                .values()
                .filter(|b| b.sheltered && b.occupied_months.contains(&month))
                .map(|b| b.area)
                .sum();
            BedUsagePoint {
                month,
                all_pct: pct(occ_all, total_all),
                sheltered_pct: pct(occ_sheltered, total_sheltered),
            }
        })
        .collect();

    Ok(series)
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

/// Set of month-of-year values (1..=12) covered by the inclusive date range
/// `[start, end]`. Caps at all twelve once the span reaches a full year.
fn months_between(start: NaiveDate, end: NaiveDate) -> HashSet<u32> {
    let mut months = HashSet::new();
    // Walk month by month from the 1st of `start`'s month through `end`.
    let mut cur = NaiveDate::from_ymd_opt(start.year(), start.month(), 1).unwrap_or(start);
    let mut guard = 0;
    while cur <= end && guard < 14 {
        months.insert(cur.month());
        let Some(next) = cur.checked_add_months(Months::new(1)) else {
            break;
        };
        cur = next;
        guard += 1;
    }
    months
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

    #[tokio::test]
    async fn months_between_spans_inclusive() {
        let m = months_between(d(2026, 3, 10), d(2026, 6, 20));
        assert_eq!(m, [3, 4, 5, 6].into_iter().collect());
    }

    #[tokio::test]
    async fn months_between_wraps_year_end() {
        let m = months_between(d(2025, 11, 5), d(2026, 2, 15));
        assert_eq!(m, [11, 12, 1, 2].into_iter().collect());
    }

    #[tokio::test]
    async fn empty_farm_is_all_zero() {
        let r = repo().await;
        let s = bed_usage_series(&r).await.unwrap();
        assert_eq!(s.len(), 12);
        assert!(s.iter().all(|p| p.all_pct == 0.0 && p.sheltered_pct == 0.0));
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
        // empty). We add two 10 m² leaves → total bed area = 40 m².
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

        let s = bed_usage_series(&r).await.unwrap();
        let at = |m: u32| s.iter().find(|p| p.month == m).unwrap();

        // June: only the 10 m² open bed occupied → 10/40 = 25% of all beds,
        // 0% of the sheltered subset.
        assert!((at(6).all_pct - 25.0).abs() < 1e-6);
        assert!((at(6).sheltered_pct - 0.0).abs() < 1e-6);
        // February: only the 10 m² sheltered bed → 10/40 = 25% of all beds,
        // and it is the only sheltered bed → 100% of the sheltered subset.
        assert!((at(2).all_pct - 25.0).abs() < 1e-6);
        assert!((at(2).sheltered_pct - 100.0).abs() < 1e-6);
        // September: nothing growing.
        assert!((at(9).all_pct - 0.0).abs() < 1e-6);
    }
}
