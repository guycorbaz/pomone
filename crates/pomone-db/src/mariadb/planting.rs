//! `PlantingRepo` implementation for MariaDB.

use crate::codec::{
    decode_planting_schedule, decode_planting_status, encode_planting_schedule,
    encode_planting_status,
};
use crate::error::{DbError, DbResult};
use crate::mariadb::MariaDbRepository;
use crate::repository::PlantingRepo;
use async_trait::async_trait;
use pomone_domain::{LocationId, Planting, PlantingId, VarietyId};
use rust_decimal::Decimal;
use sqlx::Row;
use uuid::Uuid;

const COLUMNS: &str = "id, variety_id, location_id, area_m2, plants_count, name, notes, \
                       schedule_kind, sown_on, transplanted_on, first_harvest_on, \
                       last_harvest_on, established_on, expected_removal_on, status";

#[async_trait]
impl PlantingRepo for MariaDbRepository {
    async fn planting_get(&self, id: PlantingId) -> DbResult<Option<Planting>> {
        let sql = format!("SELECT {COLUMNS} FROM planting WHERE id = ?");
        let row = sqlx::query(&sql)
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await?;
        row.map(row_to_planting).transpose()
    }

    async fn planting_list(&self) -> DbResult<Vec<Planting>> {
        let sql = format!("SELECT {COLUMNS} FROM planting ORDER BY name");
        let rows = sqlx::query(&sql).fetch_all(&self.pool).await?;
        rows.into_iter().map(row_to_planting).collect()
    }

    async fn planting_list_for_location(&self, location_id: LocationId) -> DbResult<Vec<Planting>> {
        let sql = format!("SELECT {COLUMNS} FROM planting WHERE location_id = ? ORDER BY name");
        let rows = sqlx::query(&sql)
            .bind(location_id.as_uuid())
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(row_to_planting).collect()
    }

    async fn planting_create(&self, p: &Planting) -> DbResult<()> {
        let s = encode_planting_schedule(p.schedule);
        sqlx::query(
            "INSERT INTO planting (id, variety_id, location_id, area_m2, plants_count, name, \
             notes, schedule_kind, sown_on, transplanted_on, first_harvest_on, last_harvest_on, \
             established_on, expected_removal_on, status) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(p.id.as_uuid())
        .bind(p.variety_id.as_uuid())
        .bind(p.location_id.as_uuid())
        .bind(p.area_m2)
        .bind(i64::from(p.plants_count))
        .bind(p.name.as_deref())
        .bind(p.notes.as_deref())
        .bind(s.kind)
        .bind(s.sown_on)
        .bind(s.transplanted_on)
        .bind(s.first_harvest_on)
        .bind(s.last_harvest_on)
        .bind(s.established_on)
        .bind(s.expected_removal_on)
        .bind(encode_planting_status(p.status))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn planting_update(&self, p: &Planting) -> DbResult<()> {
        let s = encode_planting_schedule(p.schedule);
        let result = sqlx::query(
            "UPDATE planting SET variety_id = ?, location_id = ?, area_m2 = ?, \
             plants_count = ?, name = ?, notes = ?, schedule_kind = ?, sown_on = ?, \
             transplanted_on = ?, first_harvest_on = ?, last_harvest_on = ?, \
             established_on = ?, expected_removal_on = ?, status = ? WHERE id = ?",
        )
        .bind(p.variety_id.as_uuid())
        .bind(p.location_id.as_uuid())
        .bind(p.area_m2)
        .bind(i64::from(p.plants_count))
        .bind(p.name.as_deref())
        .bind(p.notes.as_deref())
        .bind(s.kind)
        .bind(s.sown_on)
        .bind(s.transplanted_on)
        .bind(s.first_harvest_on)
        .bind(s.last_harvest_on)
        .bind(s.established_on)
        .bind(s.expected_removal_on)
        .bind(encode_planting_status(p.status))
        .bind(p.id.as_uuid())
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(DbError::NotFound {
                kind: "planting",
                id: p.id.to_string(),
            });
        }
        Ok(())
    }

    async fn planting_delete(&self, id: PlantingId) -> DbResult<()> {
        let result = sqlx::query("DELETE FROM planting WHERE id = ?")
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(DbError::NotFound {
                kind: "planting",
                id: id.to_string(),
            });
        }
        Ok(())
    }
}

fn row_to_planting(row: sqlx::mysql::MySqlRow) -> DbResult<Planting> {
    let id: Uuid = row.try_get("id")?;
    let variety_id: Uuid = row.try_get("variety_id")?;
    let location_id: Uuid = row.try_get("location_id")?;
    let plants_count: i64 = row.try_get("plants_count")?;
    let plants_count = u32::try_from(plants_count)
        .map_err(|_| DbError::Malformed(format!("plants_count out of range: {plants_count}")))?;
    let kind: String = row.try_get("schedule_kind")?;
    let schedule = decode_planting_schedule(
        &kind,
        row.try_get("sown_on")?,
        row.try_get("transplanted_on")?,
        row.try_get("first_harvest_on")?,
        row.try_get("last_harvest_on")?,
        row.try_get("established_on")?,
        row.try_get("expected_removal_on")?,
    )?;
    let status: String = row.try_get("status")?;
    Ok(Planting {
        id: PlantingId::from(id),
        variety_id: VarietyId::from(variety_id),
        location_id: LocationId::from(location_id),
        area_m2: row.try_get::<Decimal, _>("area_m2")?,
        plants_count,
        schedule,
        status: decode_planting_status(&status)?,
        name: row.try_get("name")?,
        notes: row.try_get("notes")?,
    })
}
