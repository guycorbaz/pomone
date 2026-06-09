//! Crop Map: bed-occupancy view across the 12-month season.
//!
//! Mirrors Qrop's `LocationView.qml`. Each `Location` becomes a horizontal
//! "lane"; every `Planting` attached to it renders as a colored bar
//! positioned by its day-of-year span. The UI tree on the left mirrors
//! the location hierarchy so children stack under their parent.
//!
//! Two write paths sit alongside the read view:
//!
//! - [`move_planting_to_location`] — moves a planting to another bed
//!   (the FK update is the entire operation; `task` and `yearly_harvest`
//!   rows follow via their own planting_id FK).
//! - [`split_planting`] — divides one planting across several beds. The
//!   first split mutates the source row in place (preserving its tasks /
//!   harvests history); the remaining splits become fresh plantings that
//!   share the same variety + schedule but start with no operational
//!   trail. Caller-supplied `(area, count)` values; sum-validation is
//!   left to the user (Qrop does the same).

use crate::error::{AppError, AppResult};
use crate::plantings_view::parse_id;
use chrono::{Datelike, NaiveDate};
use pomone_db::Repository;
use pomone_domain::{Location, LocationId, Planting, PlantingId, PlantingSchedule, VarietyId};
use rust_decimal::Decimal;
use std::collections::{HashMap, HashSet};

/// One bar on the crop map: a single planting positioned along the
/// 12-month axis (1..=365 day-of-year). Perennial plantings span the
/// whole year (1..365) so they remain visible whatever the season window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CropMapBar {
    pub planting_id: String,
    /// Compact label (`"Crop · Variety"`), trimmed.
    pub label: String,
    /// Resolved hex color derived deterministically from the variety's
    /// family id (so the same family is always the same color across
    /// sessions even though `Family` has no color field).
    pub color_hex: String,
    /// `1..=366` — start of the visible span on this lane.
    pub start_doy: i32,
    /// `1..=366`, inclusive — end of the visible span.
    pub end_doy: i32,
}

/// One row of the crop map: a single Location and the bars sitting on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CropMapLane {
    pub location_id: String,
    /// `"Parent / Child"` when nested, just `"Name"` at the root.
    pub label: String,
    /// `"L × W m²"` summary, ready for display next to the name.
    pub dimensions_label: String,
    pub bars: Vec<CropMapBar>,
}

/// Build the full Crop Map: every Location becomes a lane, with the
/// plantings that occupy it.
///
/// One round-trip per lookup table (locations / plantings / varieties /
/// crops / families) — same shape as the other `*_view` helpers, fine
/// at Pomone's scale.
pub async fn list_crop_map_lanes(repo: &dyn Repository) -> AppResult<Vec<CropMapLane>> {
    let locations = repo.location_list().await?;
    let plantings = repo.planting_list().await?;
    let varieties = repo.variety_list().await?;
    let crops = repo.crop_list().await?;

    let loc_by_id: HashMap<LocationId, &Location> = locations.iter().map(|l| (l.id, l)).collect();
    let var_by_id: HashMap<VarietyId, _> = varieties.iter().map(|v| (v.id, v)).collect();
    let crop_by_id: HashMap<_, _> = crops.iter().map(|c| (c.id, c)).collect();

    // Group plantings by location.
    let mut by_location: HashMap<LocationId, Vec<&Planting>> = HashMap::new();
    for p in &plantings {
        by_location.entry(p.location_id).or_default().push(p);
    }

    let mut lanes: Vec<CropMapLane> = locations
        .iter()
        .map(|l| {
            let parent_name = l
                .parent_id
                .and_then(|p| loc_by_id.get(&p))
                .map(|p| p.name.as_str());
            let label = match parent_name {
                Some(p) => format!("{p} / {}", l.name),
                None => l.name.clone(),
            };
            let dimensions_label =
                format!("{} × {} m", normalize(l.length_m), normalize(l.width_m));
            let bars = by_location
                .get(&l.id)
                .map(|plantings| {
                    let mut v: Vec<CropMapBar> = plantings
                        .iter()
                        .map(|p| {
                            let variety = var_by_id.get(&p.variety_id);
                            let (crop_name, family_id) = variety.map_or_else(
                                || ("?".to_owned(), None),
                                |v| {
                                    let c = crop_by_id.get(&v.crop_id);
                                    (
                                        c.map_or("?", |c| c.name.as_str()).to_owned(),
                                        c.map(|c| c.family_id),
                                    )
                                },
                            );
                            let v_name = variety.map_or("?", |v| v.name.as_str());
                            let (start, end) = doy_span(p);
                            CropMapBar {
                                planting_id: p.id.to_string(),
                                label: format!("{crop_name} · {v_name}"),
                                color_hex: family_color(family_id),
                                start_doy: start,
                                end_doy: end,
                            }
                        })
                        .collect();
                    v.sort_by_key(|b| b.start_doy);
                    v
                })
                .unwrap_or_default();
            CropMapLane {
                location_id: l.id.to_string(),
                label,
                dimensions_label,
                bars,
            }
        })
        .collect();
    // Stable order: parent-first (no parent) then alphabetical within a level.
    lanes.sort_by(|a, b| a.label.cmp(&b.label));
    Ok(lanes)
}

/// Move a planting to another location. Existing tasks / yearly harvests
/// stay attached to the same planting (their FK is on `planting_id`, not
/// on `location_id`), which is the desired behavior — the operational
/// history follows the planting wherever it goes.
pub async fn move_planting_to_location(
    repo: &dyn Repository,
    planting_id_str: &str,
    new_location_id_str: &str,
) -> AppResult<()> {
    let planting_id: PlantingId = parse_id(planting_id_str)?;
    let new_location_id: LocationId = parse_id(new_location_id_str)?;
    let mut planting = repo
        .planting_get(planting_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            kind: "planting",
            id: planting_id_str.to_owned(),
        })?;
    // Confirm the target exists (better UX than a generic FK error).
    if repo.location_get(new_location_id).await?.is_none() {
        return Err(AppError::NotFound {
            kind: "location",
            id: new_location_id_str.to_owned(),
        });
    }
    planting.location_id = new_location_id;
    repo.planting_update(&planting).await?;
    Ok(())
}

/// One part of a [`split_planting`] request: where to put it, with how
/// much area and how many plants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitPart {
    pub location_id: String,
    pub area_m2: Decimal,
    pub plants_count: u32,
}

/// Split one planting into several. The **first** part mutates the source
/// planting in place — its operational history (tasks, yearly harvests)
/// stays attached. Subsequent parts create fresh plantings sharing the
/// source's variety + schedule + name + notes but with their own area /
/// plants_count on the requested location.
///
/// Returns the list of resulting planting IDs (source first, then the new
/// ones in order). The caller may use them to navigate.
///
/// `parts` must have at least 2 entries — calling with 1 part is just a
/// `move_planting_to_location` (and is rejected here so the caller picks
/// the right entry point).
pub async fn split_planting(
    repo: &dyn Repository,
    source_id_str: &str,
    parts: &[SplitPart],
) -> AppResult<Vec<String>> {
    if parts.len() < 2 {
        return Err(AppError::Inconsistent(
            "split needs at least 2 parts".to_owned(),
        ));
    }
    let source_id: PlantingId = parse_id(source_id_str)?;
    let mut source = repo
        .planting_get(source_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            kind: "planting",
            id: source_id_str.to_owned(),
        })?;
    // We need the crop's lifespan to call `Planting::new` for the extra
    // parts (the schedule must be compatible with it).
    let variety = repo
        .variety_get(source.variety_id)
        .await?
        .ok_or_else(|| AppError::Inconsistent("planting refers to unknown variety".into()))?;
    let crop = repo
        .crop_get(variety.crop_id)
        .await?
        .ok_or_else(|| AppError::Inconsistent("variety refers to unknown crop".into()))?;

    // Pre-flight: validate every target location exists. Failing late
    // after one INSERT would leave the user with a partial split.
    let mut target_ids: Vec<LocationId> = Vec::with_capacity(parts.len());
    let mut seen: HashSet<LocationId> = HashSet::new();
    for p in parts {
        let id: LocationId = parse_id(&p.location_id)?;
        if !seen.insert(id) {
            return Err(AppError::Inconsistent(format!(
                "duplicate target location in split: {}",
                p.location_id
            )));
        }
        if repo.location_get(id).await?.is_none() {
            return Err(AppError::NotFound {
                kind: "location",
                id: p.location_id.clone(),
            });
        }
        target_ids.push(id);
    }

    let mut produced: Vec<String> = Vec::with_capacity(parts.len());

    // First part → update the source in place.
    source.location_id = target_ids[0];
    source.area_m2 = parts[0].area_m2;
    source.plants_count = parts[0].plants_count;
    repo.planting_update(&source).await?;
    produced.push(source.id.to_string());

    // Subsequent parts → create fresh plantings using the domain
    // constructor (so the validation rules apply).
    for (idx, p) in parts.iter().enumerate().skip(1) {
        let fresh = Planting::new(
            source.variety_id,
            target_ids[idx],
            source.strata_id,
            crop.lifespan,
            p.area_m2,
            p.plants_count,
            source.schedule,
            source.name.clone(),
            source.notes.clone(),
        )?;
        repo.planting_create(&fresh).await?;
        produced.push(fresh.id.to_string());
    }
    Ok(produced)
}

/// Strip trailing zeros from a `Decimal` for display ("20.000" → "20").
fn normalize(d: Decimal) -> String {
    d.normalize().to_string()
}

/// Day-of-year span for the bar:
/// - `Cycle`: from the earliest known date (sown / transplanted / first
///   harvest) to `last_harvest_on`.
/// - `Perennial`: full year (1..=365) — the crop map shows a perennial as
///   a permanent occupant of its bed, which is the operational reality.
fn doy_span(p: &Planting) -> (i32, i32) {
    match p.schedule {
        PlantingSchedule::Cycle {
            sown_on,
            transplanted_on,
            first_harvest_on,
            last_harvest_on,
        } => {
            let candidates: Vec<NaiveDate> = [sown_on, transplanted_on, Some(first_harvest_on)]
                .into_iter()
                .flatten()
                .collect();
            let start = candidates
                .iter()
                .min()
                .copied()
                .unwrap_or(first_harvest_on)
                .ordinal();
            let end = last_harvest_on.ordinal();
            #[allow(clippy::cast_possible_wrap)]
            (start as i32, end as i32)
        }
        PlantingSchedule::Perennial { .. } => (1, 365),
    }
}

/// Deterministic per-family color: small palette indexed by a hash of the
/// family UUID. When the family is unknown (orphan variety), falls back
/// to a neutral grey. Hex form so the UI uses the existing `parse_hex_color`.
fn family_color(family_id: Option<pomone_domain::FamilyId>) -> String {
    const PALETTE: &[&str] = &[
        "#3C6E47", "#B85C38", "#6FAF7A", "#B07C25", "#5F9F8B", "#A64238", "#6B5D4D", "#244529",
        "#C8B89A", "#7A6A5C", "#9A6E5C", "#4F7F8F",
    ];
    match family_id {
        Some(fid) => {
            let uuid = fid.as_uuid();
            let bytes = uuid.as_bytes();
            let idx = (u32::from(bytes[0]) ^ u32::from(bytes[15])) as usize % PALETTE.len();
            PALETTE[idx].to_owned()
        }
        None => "#A09887".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::create_annual_planting_from_sowing;
    use crate::test_helpers::seed_test_data;
    use pomone_db::{seed_defaults, LocationRepo, SqliteRepository, StrataRepo, VarietyRepo};
    use rust_decimal_macros::dec;

    async fn fresh_with_planting() -> (SqliteRepository, String, String, String) {
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        seed_test_data(&repo).await.unwrap();
        let varieties = repo.variety_list().await.unwrap();
        let locations = repo.location_list().await.unwrap();
        let bed_a = locations.iter().find(|l| l.parent_id.is_some()).unwrap();
        // Need a second bed under the same parent for move/split tests —
        // create one programmatically so the test data stays minimal.
        let bed_b = pomone_domain::Location::new(
            bed_a.kind_id,
            "Planche B",
            dec!(10),
            dec!(1),
            bed_a.parent_id,
            None,
        )
        .unwrap();
        repo.location_create(&bed_b).await.unwrap();

        let planting = create_annual_planting_from_sowing(
            &repo,
            varieties[0].id,
            bed_a.id,
            repo.strata_list().await.unwrap()[0].id,
            NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            dec!(20),
            100,
            None,
            None,
        )
        .await
        .unwrap();
        (
            repo,
            planting.id.to_string(),
            bed_a.id.to_string(),
            bed_b.id.to_string(),
        )
    }

    #[tokio::test]
    async fn list_lanes_returns_one_per_location() {
        let (repo, planting_id, bed_a, _bed_b) = fresh_with_planting().await;
        let lanes = list_crop_map_lanes(&repo).await.unwrap();
        // 2 beds + the parent location (root) = 3 lanes.
        assert!(lanes.len() >= 2);
        let lane_a = lanes.iter().find(|l| l.location_id == bed_a).unwrap();
        assert_eq!(lane_a.bars.len(), 1);
        assert_eq!(lane_a.bars[0].planting_id, planting_id);
        assert!(lane_a.bars[0].label.starts_with("Tomate · "));
        assert!(lane_a.bars[0].color_hex.starts_with('#'));
        // Day-of-year for March 1, 2026 is 60.
        assert_eq!(lane_a.bars[0].start_doy, 60);
    }

    #[tokio::test]
    async fn move_planting_changes_its_location() {
        let (repo, planting_id, _bed_a, bed_b) = fresh_with_planting().await;
        move_planting_to_location(&repo, &planting_id, &bed_b)
            .await
            .unwrap();
        let lanes = list_crop_map_lanes(&repo).await.unwrap();
        let lane_b = lanes.iter().find(|l| l.location_id == bed_b).unwrap();
        assert_eq!(lane_b.bars.len(), 1);
        assert_eq!(lane_b.bars[0].planting_id, planting_id);
    }

    #[tokio::test]
    async fn move_to_unknown_location_returns_not_found() {
        let (repo, planting_id, _bed_a, _bed_b) = fresh_with_planting().await;
        let fake = uuid::Uuid::new_v4().to_string();
        let err = move_planting_to_location(&repo, &planting_id, &fake)
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            AppError::NotFound {
                kind: "location",
                ..
            }
        ));
    }

    #[tokio::test]
    async fn split_keeps_source_id_and_creates_extras() {
        let (repo, planting_id, bed_a, bed_b) = fresh_with_planting().await;
        let parts = vec![
            SplitPart {
                location_id: bed_a.clone(),
                area_m2: dec!(8),
                plants_count: 40,
            },
            SplitPart {
                location_id: bed_b.clone(),
                area_m2: dec!(12),
                plants_count: 60,
            },
        ];
        let produced = split_planting(&repo, &planting_id, &parts).await.unwrap();
        assert_eq!(produced.len(), 2);
        assert_eq!(produced[0], planting_id); // source preserved
        let lanes = list_crop_map_lanes(&repo).await.unwrap();
        let lane_a = lanes.iter().find(|l| l.location_id == bed_a).unwrap();
        let lane_b = lanes.iter().find(|l| l.location_id == bed_b).unwrap();
        assert_eq!(lane_a.bars.len(), 1);
        assert_eq!(lane_b.bars.len(), 1);
        assert_eq!(lane_a.bars[0].planting_id, planting_id);
        assert_ne!(lane_b.bars[0].planting_id, planting_id);
    }

    #[tokio::test]
    async fn split_rejects_single_part() {
        let (repo, planting_id, bed_a, _bed_b) = fresh_with_planting().await;
        let parts = vec![SplitPart {
            location_id: bed_a,
            area_m2: dec!(20),
            plants_count: 100,
        }];
        let err = split_planting(&repo, &planting_id, &parts)
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Inconsistent(_)));
    }

    #[tokio::test]
    async fn split_rejects_duplicate_targets() {
        let (repo, planting_id, bed_a, _bed_b) = fresh_with_planting().await;
        let parts = vec![
            SplitPart {
                location_id: bed_a.clone(),
                area_m2: dec!(10),
                plants_count: 50,
            },
            SplitPart {
                location_id: bed_a,
                area_m2: dec!(10),
                plants_count: 50,
            },
        ];
        let err = split_planting(&repo, &planting_id, &parts)
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Inconsistent(_)));
    }
}
