//! Presentation-layer helper for the **plan grid** (Epic 2, story 2.3).
//!
//! The grid is a spreadsheet-faithful editor over [`CropPlanLine`]s: each row is
//! a plain-string [`PlanRow`] the Slint grid renders and edits cell by cell, and
//! this module parses the edited strings back, validates them (locale decimals,
//! ISO dates) and persists through [`CropPlanLineRepo`]. The **derived** date
//! columns (the succession dates) are computed here and rendered read-only by
//! the grid.
//!
//! Cell-level validation is exposed as small `parse_*` helpers the wiring calls
//! on cell exit — an invalid value keeps the cell open (the caller doesn't
//! commit), Échap restores. Whole-row persistence goes through [`save_plan_line`].

use crate::error::{AppError, AppResult};
use crate::i18n::I18n;
use crate::plantings_view::parse_id;
use chrono::NaiveDate;
use pomone_db::Repository;
use pomone_domain::{CropPlanLine, CropPlanLineId, VarietyId};
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::str::FromStr;

/// A plan-grid row: every cell as a UI-ready string. Editable cells are the
/// first block; `variety_label` and `derived_dates` are display-only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanRow {
    pub id: String,
    /// Stringified `VarietyId` (the editable value the picker sets).
    pub variety_id: String,
    /// `"Crop · Variety"` shown in the (read-back) variety cell.
    pub variety_label: String,
    pub series: String,
    pub bed_meters: String,
    pub stagger_days: String,
    /// ISO `YYYY-MM-DD` or empty.
    pub first_on: String,
    pub draft: bool,
    pub notes: String,
    /// Derived, read-only: the succession dates as a compact `first → last`
    /// (or the single date, or empty when `first_on` is unset).
    pub derived_dates: String,
    /// How many planned plantings this line **has** generated (story 2.6), as a
    /// string for the grid's «Generate (N)» affordance. Reflects the actual rows,
    /// so after editing `series` it lags until the grower re-generates — the
    /// count means "generated so far", not "will generate".
    pub generated_count: String,
    /// Derived, read-only (story 2.7): this line's total quantity in bed-meters,
    /// `series × bed_meters`, exact. The grid's "needs" figure — it feeds the
    /// per-variety aggregation in the «Besoins» view.
    pub need_total: String,
}

/// List every plan line as a grid row, newest-anchored first then by id for a
/// stable order. One DB read per lookup table.
pub async fn list_plan_rows(repo: &dyn Repository, _i18n: &I18n) -> AppResult<Vec<PlanRow>> {
    let lines = repo.crop_plan_line_list().await?;
    let varieties = repo.variety_list().await?;
    let crops = repo.crop_list().await?;
    let var_by_id: HashMap<_, _> = varieties.iter().map(|v| (v.id, v)).collect();
    let crop_by_id: HashMap<_, _> = crops.iter().map(|c| (c.id, c)).collect();

    // Generated-planting counts per line (story 2.6), one read for the lot.
    let mut generated: HashMap<pomone_domain::CropPlanLineId, usize> = HashMap::new();
    for pp in repo.planned_planting_list_all().await? {
        *generated.entry(pp.crop_plan_line_id).or_default() += 1;
    }

    Ok(lines
        .into_iter()
        .map(|line| {
            let count = generated.get(&line.id).copied().unwrap_or(0);
            plan_row(&line, &var_by_id, &crop_by_id, count)
        })
        .collect())
}

/// Fields for creating or updating a plan line from the grid. All strings —
/// exactly what the cells hold. `id` empty = create; set = update in place.
#[derive(Debug, Clone, Default)]
pub struct PlanLineInput {
    pub id: String,
    pub variety_id: String,
    pub series: String,
    pub bed_meters: String,
    pub stagger_days: String,
    pub first_on: String,
    pub draft: bool,
    pub notes: String,
}

/// Parse + validate the input and persist (create or update). Returns the line
/// id. Invalid cells surface as [`AppError::Inconsistent`] with a localizable
/// sentinel-free message (the grid already validates per-cell; this is the
/// backstop that also enforces the domain invariants).
pub async fn save_plan_line(repo: &dyn Repository, input: &PlanLineInput) -> AppResult<String> {
    let variety_id: VarietyId = parse_id(&input.variety_id)?;
    let series = parse_count(&input.series, "series")?;
    let bed_meters = parse_decimal_locale(&input.bed_meters, "bed_meters")?;
    let stagger_days = parse_count(&input.stagger_days, "stagger_days")?;
    let first_on = parse_optional_date(&input.first_on)?;
    let notes = empty_to_none(&input.notes);

    if input.id.trim().is_empty() {
        let line = CropPlanLine::new(
            variety_id,
            series,
            bed_meters,
            stagger_days,
            first_on,
            input.draft,
            notes,
        )?;
        let id = line.id.to_string();
        repo.crop_plan_line_create(&line).await?;
        Ok(id)
    } else {
        let id: CropPlanLineId = parse_id(&input.id)?;
        let existing = repo
            .crop_plan_line_get(id)
            .await?
            .ok_or_else(|| AppError::NotFound {
                kind: "crop_plan_line",
                id: input.id.clone(),
            })?;
        let updated = existing.with_updates(
            variety_id,
            series,
            bed_meters,
            stagger_days,
            first_on,
            input.draft,
            notes,
        )?;
        repo.crop_plan_line_update(&updated).await?;
        Ok(id.to_string())
    }
}

/// Delete a plan line by id.
pub async fn delete_plan_line(repo: &dyn Repository, id_str: &str) -> AppResult<()> {
    let id: CropPlanLineId = parse_id(id_str)?;
    repo.crop_plan_line_delete(id).await?;
    Ok(())
}

/// Duplicate a plan line (story 2.4): a faithful copy with a fresh id, so the
/// grower edits the copy («duplicates last month's lettuce line and edits the
/// variety and dates») instead of re-filling a blank. Returns the new line's id.
pub async fn duplicate_plan_line(repo: &dyn Repository, id_str: &str) -> AppResult<String> {
    let id: CropPlanLineId = parse_id(id_str)?;
    let source = repo
        .crop_plan_line_get(id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            kind: "crop_plan_line",
            id: id_str.to_owned(),
        })?;
    // A brand-new id via `new`, every other field copied verbatim.
    let copy = CropPlanLine::new(
        source.variety_id,
        source.series,
        source.bed_meters,
        source.stagger_days,
        source.first_on,
        source.draft,
        source.notes.clone(),
    )?;
    let new_id = copy.id.to_string();
    repo.crop_plan_line_create(&copy).await?;
    Ok(new_id)
}

// --- cell-level parsing (also used by the wiring for on-exit validation) ---

/// Parse a non-negative count (`series`, `stagger_days`) — a plain base-10
/// `u32`. Blank or non-numeric is rejected.
pub fn parse_count(s: &str, field: &'static str) -> AppResult<u32> {
    s.trim()
        .parse::<u32>()
        .map_err(|_| AppError::Inconsistent(format!("invalid {field}: '{s}'")))
}

/// Parse a decimal accepting the locale comma **or** period as the separator
/// (`"1,5"` and `"1.5"` both parse to 1.5) — the grid must feel like Calc.
pub fn parse_decimal_locale(s: &str, field: &'static str) -> AppResult<Decimal> {
    let normalized = s.trim().replace(',', ".");
    Decimal::from_str(&normalized)
        .map_err(|_| AppError::Inconsistent(format!("invalid {field}: '{s}'")))
}

/// Parse an optional ISO date: blank → `None`, else `YYYY-MM-DD`.
pub fn parse_optional_date(s: &str) -> AppResult<Option<NaiveDate>> {
    let t = s.trim();
    if t.is_empty() {
        return Ok(None);
    }
    NaiveDate::parse_from_str(t, "%Y-%m-%d")
        .map(Some)
        .map_err(|_| AppError::Inconsistent(format!("invalid date (want YYYY-MM-DD): '{s}'")))
}

// --- internals ---

fn empty_to_none(s: &str) -> Option<String> {
    let t = s.trim();
    (!t.is_empty()).then(|| t.to_owned())
}

fn plan_row(
    line: &CropPlanLine,
    var_by_id: &HashMap<VarietyId, &pomone_domain::Variety>,
    crop_by_id: &HashMap<pomone_domain::CropId, &pomone_domain::Crop>,
    generated_count: usize,
) -> PlanRow {
    let variety_label = var_by_id.get(&line.variety_id).map_or_else(
        || "?".to_owned(),
        |v| variety_label(v.crop_id, crop_by_id, &v.name),
    );
    PlanRow {
        id: line.id.to_string(),
        variety_id: line.variety_id.to_string(),
        variety_label,
        series: line.series.to_string(),
        bed_meters: line.bed_meters.normalize().to_string(),
        stagger_days: line.stagger_days.to_string(),
        first_on: line
            .first_on
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_default(),
        draft: line.draft,
        notes: line.notes.clone().unwrap_or_default(),
        derived_dates: format_derived_dates(&line.succession_dates()),
        generated_count: generated_count.to_string(),
        need_total: (Decimal::from(line.series) * line.bed_meters)
            .normalize()
            .to_string(),
    }
}

fn variety_label(
    crop_id: pomone_domain::CropId,
    crop_by_id: &HashMap<pomone_domain::CropId, &pomone_domain::Crop>,
    variety_name: &str,
) -> String {
    let crop_name = crop_by_id.get(&crop_id).map_or("?", |c| c.name.as_str());
    format!("{crop_name} · {variety_name}")
}

/// Compact rendering of the succession dates for the read-only derived column:
/// empty, a single date, or `first → last`.
fn format_derived_dates(dates: &[NaiveDate]) -> String {
    match (dates.first(), dates.last()) {
        (None, _) => String::new(),
        (Some(first), Some(last)) if first == last => first.format("%Y-%m-%d").to_string(),
        (Some(first), Some(last)) => {
            format!("{} → {}", first.format("%Y-%m-%d"), last.format("%Y-%m-%d"))
        }
        (Some(first), None) => first.format("%Y-%m-%d").to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pomone_db::{seed_defaults, CropPlanLineRepo, SqliteRepository};

    async fn repo_with_variety() -> (SqliteRepository, String) {
        use pomone_db::{CropRepo, FamilyRepo, VarietyRepo};
        use pomone_domain::{
            AnnualProfile, Crop, Family, Lifespan, PruningSeason, Variety, VarietyProfile,
        };
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        let f = Family::new("Asteraceae", None, None).unwrap();
        repo.family_create(&f).await.unwrap();
        let crop = Crop::new(f.id, "Laitue", None, Lifespan::Annual, PruningSeason::None).unwrap();
        repo.crop_create(&crop).await.unwrap();
        let v = Variety::new(
            crop.id,
            Lifespan::Annual,
            "Batavia",
            None,
            VarietyProfile::Annual(AnnualProfile::new(Some(20), 45, 30).unwrap()),
        )
        .unwrap();
        repo.variety_create(&v).await.unwrap();
        (repo, v.id.to_string())
    }

    fn i18n() -> I18n {
        I18n::new(crate::i18n::Lang::Fr).unwrap()
    }

    #[test]
    fn locale_decimal_accepts_comma_and_period() {
        assert_eq!(
            parse_decimal_locale("1,5", "x").unwrap(),
            parse_decimal_locale("1.5", "x").unwrap()
        );
        assert!(parse_decimal_locale("abc", "x").is_err());
    }

    #[test]
    fn optional_date_blank_is_none_invalid_errors() {
        assert_eq!(parse_optional_date("  ").unwrap(), None);
        assert_eq!(
            parse_optional_date("2026-04-01").unwrap(),
            NaiveDate::from_ymd_opt(2026, 4, 1)
        );
        assert!(parse_optional_date("01/04/2026").is_err());
    }

    #[test]
    fn count_rejects_blank_and_non_numeric() {
        assert_eq!(parse_count("6", "series").unwrap(), 6);
        assert!(parse_count("", "series").is_err());
        assert!(parse_count("-1", "series").is_err());
        assert!(parse_count("1.5", "series").is_err());
    }

    #[tokio::test]
    async fn save_creates_then_updates_and_lists_with_derived_dates() {
        let (repo, vid) = repo_with_variety().await;
        // Create (locale comma decimal, a first date).
        let id = save_plan_line(
            &repo,
            &PlanLineInput {
                id: String::new(),
                variety_id: vid.clone(),
                series: "3".into(),
                bed_meters: "15,5".into(),
                stagger_days: "14".into(),
                first_on: "2026-04-01".into(),
                draft: true,
                notes: "  batavia  ".into(),
            },
        )
        .await
        .unwrap();

        let rows = list_plan_rows(&repo, &i18n()).await.unwrap();
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(row.series, "3");
        assert_eq!(row.bed_meters, "15.5");
        assert!(row.draft);
        assert_eq!(row.notes, "batavia");
        assert!(row.variety_label.ends_with("Batavia"));
        // 3 successions × 14 days from 2026-04-01 → last 2026-04-29.
        assert_eq!(row.derived_dates, "2026-04-01 → 2026-04-29");
        // Needs figure (story 2.7): 3 series × 15.5 m = 46.5 bed-meters, exact.
        assert_eq!(row.need_total, "46.5");

        // Update: promote from draft, clear the date → no derived dates.
        save_plan_line(
            &repo,
            &PlanLineInput {
                id: id.clone(),
                variety_id: vid,
                series: "3".into(),
                bed_meters: "15,5".into(),
                stagger_days: "14".into(),
                first_on: String::new(),
                draft: false,
                notes: String::new(),
            },
        )
        .await
        .unwrap();
        let rows = list_plan_rows(&repo, &i18n()).await.unwrap();
        assert_eq!(rows.len(), 1, "update did not create a second line");
        assert!(!rows[0].draft);
        assert_eq!(rows[0].derived_dates, "");

        delete_plan_line(&repo, &id).await.unwrap();
        assert!(list_plan_rows(&repo, &i18n()).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn duplicate_makes_a_faithful_copy_with_a_new_id() {
        let (repo, vid) = repo_with_variety().await;
        let id = save_plan_line(
            &repo,
            &PlanLineInput {
                variety_id: vid,
                series: "6".into(),
                bed_meters: "15".into(),
                stagger_days: "14".into(),
                first_on: "2026-04-01".into(),
                draft: false,
                notes: "batavia".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();

        let new_id = duplicate_plan_line(&repo, &id).await.unwrap();
        assert_ne!(new_id, id, "the copy gets a fresh id");
        let rows = list_plan_rows(&repo, &i18n()).await.unwrap();
        assert_eq!(rows.len(), 2);
        // The two rows are identical except for their id.
        let a = rows.iter().find(|r| r.id == id).unwrap();
        let b = rows.iter().find(|r| r.id == new_id).unwrap();
        assert_eq!(a.series, b.series);
        assert_eq!(a.bed_meters, b.bed_meters);
        assert_eq!(a.first_on, b.first_on);
        assert_eq!(a.notes, b.notes);
        assert_eq!(a.variety_id, b.variety_id);
    }

    #[tokio::test]
    async fn save_rejects_invalid_series_without_persisting() {
        let (repo, vid) = repo_with_variety().await;
        let res = save_plan_line(
            &repo,
            &PlanLineInput {
                variety_id: vid,
                series: "0".into(),
                bed_meters: "10".into(),
                stagger_days: "7".into(),
                ..Default::default()
            },
        )
        .await;
        assert!(res.is_err());
        assert!(repo.crop_plan_line_list().await.unwrap().is_empty());
    }
}
