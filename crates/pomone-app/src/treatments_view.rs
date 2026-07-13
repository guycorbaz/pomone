//! Presentation-layer helpers for the per-planting treatments table.
//!
//! Phytosanitary traceability (issue #82): the Planting detail screen shows
//! the history of product applications. Mirrors the other `*_view` modules —
//! flat string DTOs only, the UI never sees `Decimal`/`Uuid`/`NaiveDate`.

use crate::error::AppResult;
use crate::plantings_view::parse_id;
use pomone_db::Repository;
use pomone_domain::{PlantingId, Treatment, TreatmentId};

/// One row of the treatments table on the Planting detail screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreatmentRow {
    pub treatment_id: String,
    /// ISO date the product was applied (`YYYY-MM-DD`).
    pub applied_on: String,
    pub active_substance: String,
    pub product_name: String,
    /// Dose and its unit rendered together, e.g. `"1.25 kg/ha"`.
    pub dose_label: String,
    pub notes: String,
}

fn to_row(t: &Treatment) -> TreatmentRow {
    TreatmentRow {
        treatment_id: t.id.to_string(),
        applied_on: t.applied_on.format("%Y-%m-%d").to_string(),
        active_substance: t.active_substance.clone(),
        product_name: t.product_name.clone(),
        dose_label: format!("{} {}", t.dose.normalize(), t.dose_unit),
        notes: t.notes.clone().unwrap_or_default(),
    }
}

/// Return the treatments recorded for `planting_id_str`, newest first.
pub async fn list_treatments_for_planting(
    repo: &dyn Repository,
    planting_id_str: &str,
) -> AppResult<Vec<TreatmentRow>> {
    let id: PlantingId = parse_id(planting_id_str)?;
    let treatments = repo.treatment_list_for_planting(id).await?;
    Ok(treatments.iter().map(to_row).collect())
}

/// Delete a treatment record. Maps the underlying DB `NotFound` to the
/// app-level variant so the UI shows a coherent message even if the record
/// vanished between the click and the call.
pub async fn delete_treatment(repo: &dyn Repository, treatment_id_str: &str) -> AppResult<()> {
    let id: TreatmentId = parse_id(treatment_id_str)?;
    repo.treatment_delete(id).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AppError;
    use crate::services::{
        create_annual_planting, record_treatment, AnnualPlantingRequest, TreatmentRequest,
    };
    use crate::test_helpers::seed_test_data;
    use chrono::NaiveDate;
    use pomone_db::{seed_defaults, LocationRepo, SqliteRepository, StrataRepo, VarietyRepo};
    use rust_decimal_macros::dec;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    async fn setup_with_planting() -> (SqliteRepository, PlantingId) {
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        seed_test_data(&repo).await.unwrap();
        let varieties = repo.variety_list().await.unwrap();
        let locations = repo.location_list().await.unwrap();
        let bed = locations.iter().find(|l| l.parent_id.is_some()).unwrap();
        let strata = repo.strata_list().await.unwrap()[0].id;
        let planting = create_annual_planting(
            &repo,
            AnnualPlantingRequest::from_sowing(
                varieties[0].id,
                bed.id,
                strata,
                d(2026, 3, 1),
                dec!(20),
                100,
            ),
        )
        .await
        .unwrap();
        (repo, planting.id)
    }

    #[tokio::test]
    async fn list_is_empty_until_a_treatment_is_recorded() {
        let (repo, pid) = setup_with_planting().await;
        let rows = list_treatments_for_planting(&repo, &pid.to_string())
            .await
            .unwrap();
        assert!(rows.is_empty());
    }

    #[tokio::test]
    async fn list_returns_rows_newest_first_with_dose_label() {
        let (repo, pid) = setup_with_planting().await;
        record_treatment(
            &repo,
            TreatmentRequest::new(
                pid,
                d(2026, 5, 10),
                "cuivre",
                "Bouillie bordelaise",
                dec!(1.250),
                "kg/ha",
            )
            .with_notes("avant pluie"),
        )
        .await
        .unwrap();
        record_treatment(
            &repo,
            TreatmentRequest::new(pid, d(2026, 6, 20), "soufre", "Thiovit", dec!(3), "g/m²"),
        )
        .await
        .unwrap();

        let rows = list_treatments_for_planting(&repo, &pid.to_string())
            .await
            .unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].applied_on, "2026-06-20");
        assert_eq!(rows[0].product_name, "Thiovit");
        assert_eq!(rows[0].dose_label, "3 g/m²");
        assert_eq!(rows[0].notes, "");
        assert_eq!(rows[1].applied_on, "2026-05-10");
        assert_eq!(rows[1].active_substance, "cuivre");
        assert_eq!(rows[1].dose_label, "1.25 kg/ha");
        assert_eq!(rows[1].notes, "avant pluie");
    }

    #[tokio::test]
    async fn delete_removes_the_row() {
        let (repo, pid) = setup_with_planting().await;
        record_treatment(
            &repo,
            TreatmentRequest::new(
                pid,
                d(2026, 5, 10),
                "spinosad",
                "Success 4",
                dec!(0.02),
                "L/ha",
            ),
        )
        .await
        .unwrap();
        let rows = list_treatments_for_planting(&repo, &pid.to_string())
            .await
            .unwrap();
        delete_treatment(&repo, &rows[0].treatment_id)
            .await
            .unwrap();
        assert!(list_treatments_for_planting(&repo, &pid.to_string())
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn invalid_uuid_is_rejected_cleanly() {
        let repo = SqliteRepository::in_memory().await.unwrap();
        let err = list_treatments_for_planting(&repo, "not-a-uuid")
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Inconsistent(_)));
    }
}
