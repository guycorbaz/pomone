//! `PlantingRepo` implementation for SQLite.

use crate::codec::{
    decimal_from_text, decimal_to_text, decode_planting_schedule, decode_planting_status,
    encode_planting_schedule, encode_planting_status,
};
use crate::error::{DbError, DbResult};
use crate::repository::PlantingRepo;
use crate::sqlite::SqliteRepository;
use async_trait::async_trait;
use pomone_domain::{LocationId, Planting, PlantingId, VarietyId};
use sqlx::Row;
use uuid::Uuid;

const COLUMNS: &str = "id, variety_id, location_id, area_m2, plants_count, name, notes, \
                       schedule_kind, sown_on, transplanted_on, first_harvest_on, \
                       last_harvest_on, established_on, expected_removal_on, status";

#[async_trait]
impl PlantingRepo for SqliteRepository {
    async fn planting_get(&self, id: PlantingId) -> DbResult<Option<Planting>> {
        let sql = format!("SELECT {COLUMNS} FROM planting WHERE id = ?1");
        let row = sqlx::query(&sql)
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await?;
        row.map(row_to_planting).transpose()
    }

    async fn planting_list(&self) -> DbResult<Vec<Planting>> {
        let sql = format!("SELECT {COLUMNS} FROM planting ORDER BY name COLLATE NOCASE");
        let rows = sqlx::query(&sql).fetch_all(&self.pool).await?;
        rows.into_iter().map(row_to_planting).collect()
    }

    async fn planting_list_for_location(&self, location_id: LocationId) -> DbResult<Vec<Planting>> {
        let sql = format!(
            "SELECT {COLUMNS} FROM planting WHERE location_id = ?1 ORDER BY name COLLATE NOCASE"
        );
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
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
        )
        .bind(p.id.as_uuid())
        .bind(p.variety_id.as_uuid())
        .bind(p.location_id.as_uuid())
        .bind(decimal_to_text(p.area_m2))
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
            "UPDATE planting SET variety_id = ?2, location_id = ?3, area_m2 = ?4, \
             plants_count = ?5, name = ?6, notes = ?7, schedule_kind = ?8, sown_on = ?9, \
             transplanted_on = ?10, first_harvest_on = ?11, last_harvest_on = ?12, \
             established_on = ?13, expected_removal_on = ?14, status = ?15 WHERE id = ?1",
        )
        .bind(p.id.as_uuid())
        .bind(p.variety_id.as_uuid())
        .bind(p.location_id.as_uuid())
        .bind(decimal_to_text(p.area_m2))
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
        if result.rows_affected() == 0 {
            return Err(DbError::NotFound {
                kind: "planting",
                id: p.id.to_string(),
            });
        }
        Ok(())
    }

    async fn planting_delete(&self, id: PlantingId) -> DbResult<()> {
        let result = sqlx::query("DELETE FROM planting WHERE id = ?1")
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

fn row_to_planting(row: sqlx::sqlite::SqliteRow) -> DbResult<Planting> {
    let id: Uuid = row.try_get("id")?;
    let variety_id: Uuid = row.try_get("variety_id")?;
    let location_id: Uuid = row.try_get("location_id")?;
    let area_text: String = row.try_get("area_m2")?;
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
        area_m2: decimal_from_text(&area_text)?,
        plants_count,
        schedule,
        status: decode_planting_status(&status)?,
        name: row.try_get("name")?,
        notes: row.try_get("notes")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::{
        CropRepo, FamilyRepo, LocationKindRepo, LocationRepo, StrataRepo, VarietyRepo,
    };
    use chrono::NaiveDate;
    use pomone_domain::{
        AnnualProfile, Crop, Family, Lifespan, Location, LocationKind, PlantingSchedule,
        PluriannualProfile, PruningSeason, Strata, Variety, VarietyProfile,
    };
    use rust_decimal_macros::dec;

    /// Sets up a fully populated repo with one crop+variety and one location,
    /// for the given `lifespan`. Returns (repo, variety_id, location_id).
    async fn setup(lifespan: Lifespan) -> (SqliteRepository, VarietyId, LocationId) {
        let repo = SqliteRepository::in_memory().await.unwrap();
        let f = Family::new("F", None, None).unwrap();
        let s = Strata::new("S", None, None, None, 0).unwrap();
        let k = LocationKind::new("K", None).unwrap();
        repo.family_create(&f).await.unwrap();
        repo.strata_create(&s).await.unwrap();
        repo.location_kind_create(&k).await.unwrap();
        let crop = Crop::new(f.id, s.id, "C", None, lifespan, PruningSeason::None).unwrap();
        repo.crop_create(&crop).await.unwrap();
        let profile = match lifespan {
            Lifespan::Annual | Lifespan::Pluriannual { .. } if lifespan.is_annual() => {
                VarietyProfile::Annual(AnnualProfile::new(None, 60, 30).unwrap())
            }
            _ if lifespan.is_recurring() => VarietyProfile::Pluriannual(
                PluriannualProfile::new(None, None, 200, 280, None).unwrap(),
            ),
            _ => VarietyProfile::Annual(AnnualProfile::new(None, 60, 30).unwrap()),
        };
        let variety = Variety::new(crop.id, lifespan, "V", None, profile).unwrap();
        repo.variety_create(&variety).await.unwrap();
        let location = Location::new(k.id, "L", dec!(10), dec!(10), None, None).unwrap();
        repo.location_create(&location).await.unwrap();
        (repo, variety.id, location.id)
    }

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[tokio::test]
    async fn cycle_planting_roundtrip() {
        let (repo, vid, lid) = setup(Lifespan::Annual).await;
        let p = Planting::new(
            vid,
            lid,
            Lifespan::Annual,
            dec!(20),
            100,
            PlantingSchedule::cycle(
                Some(d(2026, 3, 1)),
                Some(d(2026, 5, 1)),
                d(2026, 7, 1),
                d(2026, 10, 1),
            )
            .unwrap(),
            Some("Tomates".into()),
            None,
        )
        .unwrap();
        repo.planting_create(&p).await.unwrap();
        let got = repo.planting_get(p.id).await.unwrap().unwrap();
        assert_eq!(got, p);
    }

    #[tokio::test]
    async fn perennial_planting_roundtrip() {
        let lifespan = Lifespan::perennial(40, 3).unwrap();
        let (repo, vid, lid) = setup(lifespan).await;
        let p = Planting::new(
            vid,
            lid,
            lifespan,
            dec!(50),
            10,
            PlantingSchedule::perennial(d(2026, 3, 15), Some(d(2056, 12, 31))).unwrap(),
            Some("Verger".into()),
            None,
        )
        .unwrap();
        repo.planting_create(&p).await.unwrap();
        let got = repo.planting_get(p.id).await.unwrap().unwrap();
        assert_eq!(got, p);
    }

    #[tokio::test]
    async fn list_for_location_filters() {
        let (repo, vid, lid) = setup(Lifespan::Annual).await;
        // Second location
        let kind_id = repo.location_kind_list().await.unwrap()[0].id;
        let l2 = Location::new(kind_id, "L2", dec!(10), dec!(5), None, None).unwrap();
        repo.location_create(&l2).await.unwrap();

        for (loc, label) in [(lid, "p1"), (lid, "p2"), (l2.id, "p3")] {
            let p = Planting::new(
                vid,
                loc,
                Lifespan::Annual,
                dec!(10),
                5,
                PlantingSchedule::cycle(None, None, d(2026, 7, 1), d(2026, 8, 1)).unwrap(),
                Some(label.into()),
                None,
            )
            .unwrap();
            repo.planting_create(&p).await.unwrap();
        }

        let for_l1 = repo.planting_list_for_location(lid).await.unwrap();
        assert_eq!(for_l1.len(), 2);
        let for_l2 = repo.planting_list_for_location(l2.id).await.unwrap();
        assert_eq!(for_l2.len(), 1);
    }
}
