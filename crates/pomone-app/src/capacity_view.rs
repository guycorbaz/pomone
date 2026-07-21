//! Placement capacity view — DTOs for the placement screen (story 3.2).
//!
//! Builds [`pomone_domain::capacity::Placement`] inputs from the placed
//! plantings + the location geometry/hierarchy, calls the **pure** story-3.1
//! engine, and returns string/number DTOs the UI can render directly — never a
//! `Uuid`, `Decimal`, or domain enum. Three surfaces:
//!
//! * [`unplaced_list`] — the planned successions still to place
//!   (`placed_planting_id IS NULL`);
//! * [`occupancy_curve`] — the live season curve, sheltered vs open-field bed
//!   metres, with per-group capacity + overflow flag;
//! * [`peak_composition`] — which series make up the load at a chosen week
//!   (FR11, basic).
//!
//! **Read-path defensive posture (project rule):** every `Decimal` product and
//! every `f32` cast **saturates/clamps**, never panics, on a pathological
//! persisted `bed_meters`/`width_m` or a decades-wide interval. The engine
//! already saturates its sums; this view must not reintroduce a panic.

use crate::error::AppResult;
use chrono::NaiveDate;
use pomone_db::Repository;
use pomone_domain::capacity::{self, Placement};
use pomone_domain::{is_sheltered, Location, LocationId, LocationKindId};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use std::collections::{HashMap, HashSet};

/// Number of weekly buckets in the curve (mirrors `bed_usage_view`).
const WEEKS: u32 = 52;

// ============================================================
// Unplaced list
// ============================================================

/// One unplaced planned succession, ready to place. Strings only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnplacedRow {
    /// `planned_planting` id (opaque string for the UI to echo back).
    pub id: String,
    /// "Crop · Variety" label.
    pub variety: String,
    /// ISO date of the succession.
    pub planned_on: String,
    /// Bed-metres the succession needs (normalized decimal string).
    pub bed_meters: String,
}

/// The planned successions still to place (`placed_planting_id IS NULL`),
/// plan-order then succession-order.
pub async fn unplaced_list(repo: &dyn Repository) -> AppResult<Vec<UnplacedRow>> {
    let planned = repo.planned_planting_list_all().await?;
    let labels = variety_labels(repo).await?;
    Ok(planned
        .into_iter()
        .filter(|pp| pp.placed_planting_id.is_none())
        .map(|pp| UnplacedRow {
            id: pp.id.to_string(),
            variety: labels
                .get(&pp.variety_id.to_string())
                .cloned()
                .unwrap_or_default(),
            planned_on: pp.planned_on.format("%Y-%m-%d").to_string(),
            bed_meters: pp.bed_meters.normalize().to_string(),
        })
        .collect())
}

// ============================================================
// Occupancy curve
// ============================================================

/// One weekly point of the occupancy curve, in **bed-metres**, split by cover.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OccupancyPoint {
    /// 1..=52.
    pub week: i32,
    /// Sheltered bed-metres occupied that week.
    pub covered: f32,
    /// Open-field bed-metres occupied that week.
    pub open: f32,
}

/// The live occupancy curve for the placement screen: two series (sheltered vs
/// open-field), each with its own capacity, plus a peak + overflow flag.
#[derive(Debug, Clone, PartialEq)]
pub struct OccupancyCurve {
    /// 52 weekly points, week 1 first.
    pub points: Vec<OccupancyPoint>,
    /// Total sheltered bed-metres available (Σ leaf `length_m` under cover).
    pub covered_capacity: f32,
    /// Total open-field bed-metres available.
    pub open_capacity: f32,
    /// Peak sheltered occupancy across the season.
    pub peak_covered: f32,
    /// Peak open-field occupancy across the season.
    pub peak_open: f32,
    /// True if either group's peak exceeds its capacity (amber overflow).
    pub over_capacity: bool,
    /// Whether the farm has any sheltered bed.
    pub has_covered: bool,
    /// Whether the farm has any open-field bed.
    pub has_open: bool,
}

/// Build the season occupancy curve from the placed plantings, via the pure
/// engine. `season_year` selects the calendar year of the weekly axis.
pub async fn occupancy_curve(repo: &dyn Repository, season_year: i32) -> AppResult<OccupancyCurve> {
    let placed = build_placed(repo).await?;
    let (leaves_covered, leaves_open) = leaf_capacities(repo).await?;

    // `checked_add` so an extreme `season_year` (e.g. `i32::MAX`) can never
    // overflow the `+ 1` before `from_ymd_opt` gets to reject it (the module's
    // no-panic-on-out-of-range-year contract).
    let horizon = season_year
        .checked_add(1)
        .and_then(|y| NaiveDate::from_ymd_opt(y, 1, 1));
    let (Some(season_start), Some(horizon)) = (NaiveDate::from_ymd_opt(season_year, 1, 1), horizon)
    else {
        return Ok(empty_curve(leaves_covered, leaves_open));
    };

    let placements: Vec<Placement> = placed.iter().map(|p| p.placement.clone()).collect();
    let roots: Vec<LocationId> = root_ids(repo).await?;

    let mut points = Vec::with_capacity(WEEKS as usize);
    let (mut peak_covered, mut peak_open) = (0.0_f32, 0.0_f32);
    for week in 1..=WEEKS {
        let Some(day) = week_start_date(season_start, week) else {
            continue;
        };
        // Farm-wide occupancy = Σ over disjoint roots (engine does the split).
        let (mut covered, mut open) = (Decimal::ZERO, Decimal::ZERO);
        for &root in &roots {
            let split = capacity::occupancy_at(&placements, root, day, horizon);
            covered = covered.saturating_add(split.covered);
            open = open.saturating_add(split.open);
        }
        let covered = to_f32(covered);
        let open = to_f32(open);
        peak_covered = peak_covered.max(covered);
        peak_open = peak_open.max(open);
        points.push(OccupancyPoint {
            week: i32::try_from(week).unwrap_or(0),
            covered,
            open,
        });
    }

    let covered_capacity = to_f32(leaves_covered);
    let open_capacity = to_f32(leaves_open);
    let over_capacity =
        peak_covered > covered_capacity + f32::EPSILON || peak_open > open_capacity + f32::EPSILON;

    Ok(OccupancyCurve {
        points,
        covered_capacity,
        open_capacity,
        peak_covered,
        peak_open,
        over_capacity,
        has_covered: leaves_covered > Decimal::ZERO,
        has_open: leaves_open > Decimal::ZERO,
    })
}

// ============================================================
// Peak composition (FR11, basic)
// ============================================================

/// One series composing a peak: variety, its date, its bed-metres. Strings only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompositionRow {
    pub variety: String,
    pub date: String,
    pub bed_meters: String,
}

/// The placements active in `week` of `season_year` — the series composing that
/// column of the curve (what the UI lists when a peak is clicked).
///
/// `cover_group` filters to one cover side so the list matches the peak the UI
/// is explaining: `Some(true)` = sheltered only, `Some(false)` = open-field
/// only, `None` = both. (The amber peak names a specific group, so the panel
/// must compose that same group — see FR11.)
pub async fn peak_composition(
    repo: &dyn Repository,
    season_year: i32,
    week: i32,
    cover_group: Option<bool>,
) -> AppResult<Vec<CompositionRow>> {
    let placed = build_placed(repo).await?;
    let horizon = season_year
        .checked_add(1)
        .and_then(|y| NaiveDate::from_ymd_opt(y, 1, 1));
    let (Some(season_start), Some(horizon)) = (NaiveDate::from_ymd_opt(season_year, 1, 1), horizon)
    else {
        return Ok(Vec::new());
    };
    let week_u = u32::try_from(week).unwrap_or(0);
    let Some(day) = week_start_date(season_start, week_u) else {
        return Ok(Vec::new());
    };
    Ok(placed
        .iter()
        .filter(|p| p.placement.covers(day, horizon))
        .filter(|p| cover_group.map_or(true, |c| p.placement.covered == c))
        .map(|p| CompositionRow {
            variety: p.variety.clone(),
            date: p.date.format("%Y-%m-%d").to_string(),
            bed_meters: p.bed_meters.normalize().to_string(),
        })
        .collect())
}

// ============================================================
// Internals
// ============================================================

/// A placed planting turned into an engine input, keeping the display info the
/// DTOs need.
struct PlacedInfo {
    placement: Placement,
    variety: String,
    date: NaiveDate,
    bed_meters: Decimal,
}

/// Turn every placed planting into an engine [`Placement`] + display info.
async fn build_placed(repo: &dyn Repository) -> AppResult<Vec<PlacedInfo>> {
    let locations = repo.location_list().await?;
    let kinds = repo.location_kind_list().await?;
    let plantings = repo.planting_list().await?;
    let labels = variety_labels(repo).await?;

    let loc_by_id: HashMap<LocationId, &Location> = locations.iter().map(|l| (l.id, l)).collect();
    let covered_kind: HashMap<LocationKindId, bool> =
        kinds.iter().map(|k| (k.id, k.covered)).collect();

    let mut out = Vec::with_capacity(plantings.len());
    for p in &plantings {
        let Some(bed) = loc_by_id.get(&p.location_id) else {
            continue; // planting on a vanished location — skip defensively.
        };
        // Bed-metres footprint = running metres on the bed = area ÷ bed width.
        // Both are domain-positive; division only shrinks, so no overflow. A
        // zero width is impossible (domain invariant) but guarded anyway.
        let footprint = p
            .area_m2
            .checked_div(bed.width_m)
            .filter(|d| *d > Decimal::ZERO)
            .unwrap_or(p.area_m2);
        let path = ancestors(bed.id, &loc_by_id);
        let covered = is_sheltered(path.iter().map(|id| {
            loc_by_id
                .get(id)
                .and_then(|l| covered_kind.get(&l.kind_id).copied())
                .unwrap_or(false)
        }));
        // A terminated planting stops occupying its ground on its termination
        // date (story 3.4, FR15) — the interval shortens, the row stays. Its
        // *past* occupancy is history and must still show on the curve, so this
        // is deliberately not a "filter out terminated plantings".
        let (start, end) = capacity::occupancy_window(&p.schedule, p.terminated_on)?;
        out.push(PlacedInfo {
            placement: Placement {
                path,
                bed_meters: footprint,
                covered,
                start,
                end,
            },
            variety: labels
                .get(&p.variety_id.to_string())
                .cloned()
                .unwrap_or_default(),
            date: p.schedule.start_date(),
            bed_meters: footprint,
        });
    }
    Ok(out)
}

/// The leaf beds' total bed-metres capacity (`length_m`), split covered/open.
async fn leaf_capacities(repo: &dyn Repository) -> AppResult<(Decimal, Decimal)> {
    let locations = repo.location_list().await?;
    let kinds = repo.location_kind_list().await?;
    let loc_by_id: HashMap<LocationId, &Location> = locations.iter().map(|l| (l.id, l)).collect();
    let covered_kind: HashMap<LocationKindId, bool> =
        kinds.iter().map(|k| (k.id, k.covered)).collect();
    let parents: HashSet<LocationId> = locations.iter().filter_map(|l| l.parent_id).collect();

    let (mut covered, mut open) = (Decimal::ZERO, Decimal::ZERO);
    for l in &locations {
        if parents.contains(&l.id) {
            continue; // interior node — capacity lives on the leaves.
        }
        let sheltered = is_sheltered(ancestors(l.id, &loc_by_id).iter().map(|id| {
            loc_by_id
                .get(id)
                .and_then(|loc| covered_kind.get(&loc.kind_id).copied())
                .unwrap_or(false)
        }));
        if sheltered {
            covered = covered.saturating_add(l.bed_meters());
        } else {
            open = open.saturating_add(l.bed_meters());
        }
    }
    Ok((covered, open))
}

/// The location and all its ancestors, **leaf first, root last** — the engine's
/// `path`. Bounded against a corrupt cycle.
fn ancestors(leaf: LocationId, loc_by_id: &HashMap<LocationId, &Location>) -> Vec<LocationId> {
    let mut path = Vec::new();
    let mut current = Some(leaf);
    let mut seen = HashSet::new();
    while let Some(id) = current {
        if !seen.insert(id) {
            break; // defensive: never loop on a corrupt parent chain.
        }
        path.push(id);
        current = loc_by_id.get(&id).and_then(|l| l.parent_id);
    }
    path
}

/// Root locations (no parent).
async fn root_ids(repo: &dyn Repository) -> AppResult<Vec<LocationId>> {
    Ok(repo
        .location_list()
        .await?
        .into_iter()
        .filter(|l| l.parent_id.is_none())
        .map(|l| l.id)
        .collect())
}

/// "Crop · Variety" labels keyed by variety-id string.
async fn variety_labels(repo: &dyn Repository) -> AppResult<HashMap<String, String>> {
    let crops = repo.crop_list().await?;
    let crop_name: HashMap<_, _> = crops.iter().map(|c| (c.id, c.name.clone())).collect();
    Ok(repo
        .variety_list()
        .await?
        .into_iter()
        .map(|v| {
            let label = match crop_name.get(&v.crop_id) {
                Some(cn) => format!("{cn} · {}", v.name),
                None => v.name.clone(),
            };
            (v.id.to_string(), label)
        })
        .collect())
}

/// The Monday-anchored start date of weekly bucket `week` (1..=52) within
/// `season_start`'s year: `Jan 1 + (week-1)·7` days. `None` if out of range.
fn week_start_date(season_start: NaiveDate, week: u32) -> Option<NaiveDate> {
    if week == 0 {
        return None;
    }
    season_start.checked_add_signed(chrono::Duration::days(i64::from(week - 1) * 7))
}

/// Clamp a `Decimal` to `f32`, saturating to `f32::MAX` on overflow (never NaN/panic).
fn to_f32(d: Decimal) -> f32 {
    d.to_f32().unwrap_or(f32::MAX)
}

fn empty_curve(leaves_covered: Decimal, leaves_open: Decimal) -> OccupancyCurve {
    OccupancyCurve {
        points: (1..=WEEKS)
            .map(|week| OccupancyPoint {
                week: i32::try_from(week).unwrap_or(0),
                covered: 0.0,
                open: 0.0,
            })
            .collect(),
        covered_capacity: to_f32(leaves_covered),
        open_capacity: to_f32(leaves_open),
        peak_covered: 0.0,
        peak_open: 0.0,
        over_capacity: false,
        has_covered: leaves_covered > Decimal::ZERO,
        has_open: leaves_open > Decimal::ZERO,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generation::generate_from_plan_line;
    use crate::plan_view::{save_plan_line, PlanLineInput};
    use crate::services::{create_annual_planting, AnnualPlantingRequest};
    use crate::test_helpers::seed_test_data;
    use chrono::NaiveDate;
    use pomone_db::{
        seed_defaults, LocationKindRepo, LocationRepo, PlannedPlantingRepo, PlantingRepo,
        Repository, SqliteRepository, VarietyRepo,
    };
    use pomone_domain::Location;
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;

    const SEASON: i32 = 2026;

    /// Repo seeded with defaults + the `Planche A` (25 × 0.8, open field) bed
    /// and a Tomate/Marmande annual variety.
    async fn fixture() -> SqliteRepository {
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        seed_test_data(&repo).await.unwrap();
        repo
    }

    async fn bed_and_variety(repo: &dyn Repository) -> (pomone_domain::LocationId, String, String) {
        let bed = repo
            .location_list()
            .await
            .unwrap()
            .into_iter()
            .find(|l| l.name == "Planche A")
            .unwrap();
        let variety = repo.variety_list().await.unwrap()[0].id.to_string();
        let strata = repo.strata_list().await.unwrap()[0].id.to_string();
        (bed.id, variety, strata)
    }

    async fn place_annual(repo: &dyn Repository, area_m2: Decimal, plants: u32) {
        let (bed, variety, strata) = bed_and_variety(repo).await;
        create_annual_planting(
            repo,
            AnnualPlantingRequest::from_sowing(
                crate::plantings_view::parse_id(&variety).unwrap(),
                bed,
                crate::plantings_view::parse_id(&strata).unwrap(),
                NaiveDate::from_ymd_opt(2026, 4, 1).unwrap(),
                area_m2,
                plants,
            ),
            crate::test_helpers::no_cutoff_today(),
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn empty_farm_curve_is_all_zero() {
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        let curve = occupancy_curve(&repo, SEASON).await.unwrap();
        assert_eq!(curve.points.len(), 52);
        assert!(curve
            .points
            .iter()
            .all(|p| p.covered.abs() < f32::EPSILON && p.open.abs() < f32::EPSILON));
        assert!(!curve.over_capacity);
        assert!(!curve.has_covered && !curve.has_open);
    }

    #[tokio::test]
    async fn placed_planting_shows_on_the_open_curve() {
        let repo = fixture().await;
        // 16 m² on a 0.8 m-wide bed → 20 running metres, within the 25 m bed.
        place_annual(&repo, dec!(16), 80).await;
        let curve = occupancy_curve(&repo, SEASON).await.unwrap();
        assert!(curve.has_open);
        assert!(!curve.has_covered);
        // Open-field capacity = the bed's length (25 m).
        assert!((curve.open_capacity - 25.0).abs() < 0.01);
        // Some week is occupied at 20 running metres; covered stays flat.
        assert!(
            curve.peak_open > 19.9 && curve.peak_open < 20.1,
            "peak {}",
            curve.peak_open
        );
        assert!(curve.peak_covered.abs() < f32::EPSILON);
        assert!(!curve.over_capacity, "20 m fits in a 25 m bed");
    }

    #[tokio::test]
    async fn overflow_sets_the_amber_flag() {
        let repo = fixture().await;
        // 24 m² / 0.8 m = 30 running metres > the 25 m bed → overflow.
        place_annual(&repo, dec!(24), 100).await;
        let curve = occupancy_curve(&repo, SEASON).await.unwrap();
        assert!(curve.peak_open > 29.9, "peak {}", curve.peak_open);
        assert!(curve.over_capacity, "30 m overflows a 25 m bed");
    }

    #[tokio::test]
    async fn absurd_area_saturates_not_panics() {
        let repo = fixture().await;
        // A pathological area must clamp on the f32 cast, never panic.
        place_annual(&repo, Decimal::MAX, 1).await;
        let curve = occupancy_curve(&repo, SEASON).await.unwrap();
        assert!(curve.over_capacity);
        assert!(curve.peak_open.is_finite());
    }

    #[tokio::test]
    async fn unplaced_list_filters_out_placed_rows() {
        let repo = fixture().await;
        let variety = repo.variety_list().await.unwrap()[0].id.to_string();
        let line = save_plan_line(
            &repo,
            &PlanLineInput {
                variety_id: variety,
                series: "3".into(),
                bed_meters: "15".into(),
                stagger_days: "14".into(),
                first_on: "2026-04-01".into(),
                draft: false,
                ..Default::default()
            },
        )
        .await
        .unwrap();
        generate_from_plan_line(&repo, &line).await.unwrap();

        let all = unplaced_list(&repo).await.unwrap();
        assert_eq!(all.len(), 3, "three unplaced successions");
        assert!(all[0].variety.contains('·'), "crop · variety label");
        assert_eq!(all[0].bed_meters, "15");

        // Placing one (a real planting, so the FK holds) drops it off the list.
        place_annual(&repo, dec!(16), 80).await;
        let planting_id = repo.planting_list().await.unwrap()[0].id;
        let mut pp = repo.planned_planting_list_all().await.unwrap()[0].clone();
        pp.placed_planting_id = Some(planting_id);
        repo.planned_planting_update(&pp).await.unwrap();
        assert_eq!(unplaced_list(&repo).await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn covered_bed_feeds_the_sheltered_series() {
        let repo = fixture().await;
        // A greenhouse bed under the seeded covered "Serre" kind.
        let serre = repo
            .location_kind_list()
            .await
            .unwrap()
            .into_iter()
            .find(|k| k.covered)
            .unwrap();
        let bed = Location::new(serre.id, "Serre A", dec!(20), dec!(1.0), None, None).unwrap();
        repo.location_create(&bed).await.unwrap();
        let (_open_bed, variety, strata) = bed_and_variety(&repo).await;
        create_annual_planting(
            &repo,
            AnnualPlantingRequest::from_sowing(
                crate::plantings_view::parse_id(&variety).unwrap(),
                bed.id,
                crate::plantings_view::parse_id(&strata).unwrap(),
                NaiveDate::from_ymd_opt(2026, 4, 1).unwrap(),
                dec!(12),
                60,
            ),
            crate::test_helpers::no_cutoff_today(),
        )
        .await
        .unwrap();

        let curve = occupancy_curve(&repo, SEASON).await.unwrap();
        assert!(curve.has_covered);
        assert!(curve.covered_capacity > 19.9, "20 m greenhouse bed");
        assert!(curve.peak_covered > 0.0);
    }

    #[tokio::test]
    async fn out_of_range_year_yields_an_empty_curve() {
        let repo = fixture().await;
        place_annual(&repo, dec!(16), 80).await;
        // A year chrono can't represent → the guarded empty-curve path.
        let curve = occupancy_curve(&repo, 300_000).await.unwrap();
        assert_eq!(curve.points.len(), 52);
        assert!(curve.peak_open.abs() < f32::EPSILON);
        // Capacities still reflect the beds (they don't depend on the year).
        assert!(curve.has_open);
        // Composition for an impossible year is empty, not a panic.
        assert!(peak_composition(&repo, 300_000, 1, None)
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn peak_composition_lists_active_series() {
        let repo = fixture().await;
        place_annual(&repo, dec!(16), 80).await;
        // Find a week with occupancy, then assert its composition is non-empty.
        let curve = occupancy_curve(&repo, SEASON).await.unwrap();
        let peak_week = curve
            .points
            .iter()
            .max_by(|a, b| a.open.partial_cmp(&b.open).unwrap())
            .unwrap()
            .week;
        // Open-field group: the one open planting composes it.
        let comp = peak_composition(&repo, SEASON, peak_week, Some(false))
            .await
            .unwrap();
        assert_eq!(comp.len(), 1);
        assert!(comp[0].variety.contains("Marmande"));
        // No sheltered series exist → the covered group's composition is empty
        // (the filter keeps the amber peak and its explanation on the same group).
        assert!(peak_composition(&repo, SEASON, peak_week, Some(true))
            .await
            .unwrap()
            .is_empty());
        // `None` returns both sides.
        assert_eq!(
            peak_composition(&repo, SEASON, peak_week, None)
                .await
                .unwrap()
                .len(),
            1
        );
    }
}
