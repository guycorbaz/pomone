//! `TreatmentRepo` implementation for MariaDB.

use crate::error::{DbError, DbResult};
use crate::mariadb::MariaDbRepository;
use crate::repository::TreatmentRepo;
use async_trait::async_trait;
use pomone_domain::{PlantingId, Treatment, TreatmentId};
use rust_decimal::Decimal;
use sqlx::Row;
use uuid::Uuid;

const TREATMENT_COLUMNS: &str =
    "id, planting_id, applied_on, active_substance, product_name, dose, dose_unit, notes";

#[async_trait]
impl TreatmentRepo for MariaDbRepository {
    async fn treatment_get(&self, id: TreatmentId) -> DbResult<Option<Treatment>> {
        let row = sqlx::query(&format!(
            "SELECT {TREATMENT_COLUMNS} FROM treatment WHERE id = ?"
        ))
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await?;
        row.map(row_to_treatment).transpose()
    }

    async fn treatment_list_for_planting(
        &self,
        planting_id: PlantingId,
    ) -> DbResult<Vec<Treatment>> {
        let rows = sqlx::query(&format!(
            "SELECT {TREATMENT_COLUMNS} FROM treatment \
             WHERE planting_id = ? ORDER BY applied_on DESC"
        ))
        .bind(planting_id.as_uuid())
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_treatment).collect()
    }

    async fn treatment_create(&self, t: &Treatment) -> DbResult<()> {
        sqlx::query(
            "INSERT INTO treatment \
             (id, planting_id, applied_on, active_substance, product_name, dose, dose_unit, notes) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(t.id.as_uuid())
        .bind(t.planting_id.as_uuid())
        .bind(t.applied_on)
        .bind(&t.active_substance)
        .bind(&t.product_name)
        .bind(t.dose)
        .bind(&t.dose_unit)
        .bind(t.notes.as_deref())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn treatment_delete(&self, id: TreatmentId) -> DbResult<()> {
        let result = sqlx::query("DELETE FROM treatment WHERE id = ?")
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(DbError::NotFound {
                kind: "treatment",
                id: id.to_string(),
            });
        }
        Ok(())
    }
}

fn row_to_treatment(row: sqlx::mysql::MySqlRow) -> DbResult<Treatment> {
    let id: Uuid = row.try_get("id")?;
    let planting_id: Uuid = row.try_get("planting_id")?;
    Ok(Treatment {
        id: TreatmentId::from(id),
        planting_id: PlantingId::from(planting_id),
        applied_on: row.try_get("applied_on")?,
        active_substance: row.try_get("active_substance")?,
        product_name: row.try_get("product_name")?,
        dose: row.try_get::<Decimal, _>("dose")?,
        dose_unit: row.try_get("dose_unit")?,
        notes: row.try_get("notes")?,
    })
}
