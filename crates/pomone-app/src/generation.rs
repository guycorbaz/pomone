//! Generation: a crop-plan line → its N staggered planned plantings
//! (Epic 2, story 2.6).
//!
//! A complete (non-draft, dated) line materializes one [`PlannedPlanting`] per
//! succession — stagger-dated, line-linked, **unplaced** (placement is Epic 3).
//! Regeneration after a line edit is **non-destructive**: it upserts per
//! succession index (so a persisting row keeps its id, and anything a later
//! placement links to survives) and only trims successions the shortened line
//! no longer has. Draft and dateless lines don't generate.

use crate::error::{AppError, AppResult};
use crate::plantings_view::parse_id;
use pomone_db::Repository;
use pomone_domain::{CropPlanLine, CropPlanLineId, PlannedPlanting};

/// Generate (or regenerate) the planned plantings for a plan line. Returns the
/// number of planned plantings the line now has.
///
/// Errors if the line is a **draft** (drafts are excluded from generation) or
/// has **no first date** (nothing to stagger from).
pub async fn generate_from_plan_line(repo: &dyn Repository, line_id_str: &str) -> AppResult<usize> {
    let line_id: CropPlanLineId = parse_id(line_id_str)?;
    let line = repo
        .crop_plan_line_get(line_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            kind: "crop_plan_line",
            id: line_id_str.to_owned(),
        })?;

    if line.draft {
        return Err(AppError::Inconsistent("plan_line_is_draft".to_owned()));
    }
    let dates = line.succession_dates();
    if dates.is_empty() {
        return Err(AppError::Inconsistent("plan_line_has_no_date".to_owned()));
    }

    regenerate(repo, &line, &dates).await?;
    Ok(dates.len())
}

/// Upsert one planned planting per succession, then trim any surplus.
async fn regenerate(
    repo: &dyn Repository,
    line: &CropPlanLine,
    dates: &[chrono::NaiveDate],
) -> AppResult<()> {
    let existing = repo.planned_planting_list_for_line(line.id).await?;

    for (k, date) in dates.iter().enumerate() {
        let index = u32::try_from(k).unwrap_or(u32::MAX);
        if let Some(current) = existing.iter().find(|e| e.series_index == index) {
            // Update in place — preserves the id (non-destructive).
            let mut updated = current.clone();
            updated.variety_id = line.variety_id;
            updated.planned_on = *date;
            updated.bed_meters = line.bed_meters;
            repo.planned_planting_update(&updated).await?;
        } else {
            let pp = PlannedPlanting::new(line.id, line.variety_id, index, *date, line.bed_meters)?;
            repo.planned_planting_create(&pp).await?;
        }
    }

    // Trim successions the (now shorter) line no longer has. Placement (Epic 3)
    // will add a "placed" guard here so an active/placed planting is never
    // trimmed; today no planned planting is placed, so this is safe.
    let keep = dates.len();
    for e in &existing {
        if (e.series_index as usize) >= keep {
            repo.planned_planting_delete(e.id).await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan_view::{save_plan_line, PlanLineInput};
    use pomone_db::{seed_defaults, PlannedPlantingRepo, SqliteRepository};

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

    async fn line(
        repo: &SqliteRepository,
        vid: &str,
        series: &str,
        first: &str,
        draft: bool,
    ) -> String {
        save_plan_line(
            repo,
            &PlanLineInput {
                variety_id: vid.to_owned(),
                series: series.to_owned(),
                bed_meters: "15".to_owned(),
                stagger_days: "14".to_owned(),
                first_on: first.to_owned(),
                draft,
                ..Default::default()
            },
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn generates_n_staggered_line_linked_plantings() {
        let (repo, vid) = repo_with_variety().await;
        let id = line(&repo, &vid, "6", "2026-04-01", false).await;
        let n = generate_from_plan_line(&repo, &id).await.unwrap();
        assert_eq!(n, 6);

        let line_id = pomone_domain::ids::CropPlanLineId::from(uuid::Uuid::parse_str(&id).unwrap());
        let pps = repo.planned_planting_list_for_line(line_id).await.unwrap();
        assert_eq!(pps.len(), 6);
        // Stagger-dated: 14 days apart from 2026-04-01.
        assert_eq!(
            pps[0].planned_on,
            chrono::NaiveDate::from_ymd_opt(2026, 4, 1).unwrap()
        );
        // Index 5 = first + 5·14 = +70 days.
        assert_eq!(
            pps[5].planned_on,
            chrono::NaiveDate::from_ymd_opt(2026, 6, 10).unwrap()
        );
        assert!(pps.iter().all(|p| p.crop_plan_line_id == line_id));
    }

    #[tokio::test]
    async fn regeneration_is_non_destructive_and_trims() {
        let (repo, vid) = repo_with_variety().await;
        let id = line(&repo, &vid, "6", "2026-04-01", false).await;
        generate_from_plan_line(&repo, &id).await.unwrap();
        let line_id = pomone_domain::ids::CropPlanLineId::from(uuid::Uuid::parse_str(&id).unwrap());
        let before = repo.planned_planting_list_for_line(line_id).await.unwrap();
        let ids_kept: Vec<_> = before.iter().take(4).map(|p| p.id).collect();

        // Shrink the line to 4 successions and regenerate.
        crate::plan_view::save_plan_line(
            &repo,
            &PlanLineInput {
                id: id.clone(),
                variety_id: vid,
                series: "4".into(),
                bed_meters: "20".into(),
                stagger_days: "14".into(),
                first_on: "2026-04-01".into(),
                draft: false,
                ..Default::default()
            },
        )
        .await
        .unwrap();
        let n = generate_from_plan_line(&repo, &id).await.unwrap();
        assert_eq!(n, 4);

        let after = repo.planned_planting_list_for_line(line_id).await.unwrap();
        assert_eq!(after.len(), 4, "surplus successions trimmed");
        // The 4 surviving rows kept their ids (non-destructive) and got the new
        // bed-meters.
        for (kept_id, row) in ids_kept.iter().zip(after.iter()) {
            assert_eq!(*kept_id, row.id, "id preserved across regeneration");
            assert_eq!(row.bed_meters, rust_decimal_macros::dec!(20));
        }
    }

    #[tokio::test]
    async fn draft_and_dateless_lines_do_not_generate() {
        let (repo, vid) = repo_with_variety().await;
        let draft = line(&repo, &vid, "3", "2026-04-01", true).await;
        assert!(generate_from_plan_line(&repo, &draft).await.is_err());

        let dateless = line(&repo, &vid, "3", "", false).await;
        assert!(generate_from_plan_line(&repo, &dateless).await.is_err());
    }

    #[tokio::test]
    async fn deleting_the_line_cascades_to_planned_plantings() {
        let (repo, vid) = repo_with_variety().await;
        let id = line(&repo, &vid, "3", "2026-04-01", false).await;
        generate_from_plan_line(&repo, &id).await.unwrap();
        crate::plan_view::delete_plan_line(&repo, &id)
            .await
            .unwrap();
        assert!(repo.planned_planting_list_all().await.unwrap().is_empty());
    }
}
