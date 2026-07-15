//! `ItkRepo` implementation for SQLite.

use crate::error::{DbError, DbResult};
use crate::repository::ItkRepo;
use crate::sqlite::SqliteRepository;
use async_trait::async_trait;
use pomone_domain::{
    CropId, ItkActivity, ItkActivityId, ItkTemplate, ItkTemplateId, TaskImplementId, TaskMethodId,
    TaskTypeId,
};
use sqlx::Row;
use uuid::Uuid;

const ACTIVITY_COLUMNS: &str = "id, template_id, task_type_id, offset_days, method_id, \
     implement_id, label, position, notes";

#[async_trait]
impl ItkRepo for SqliteRepository {
    async fn itk_template_get(&self, id: ItkTemplateId) -> DbResult<Option<ItkTemplate>> {
        let row = sqlx::query("SELECT id, crop_id FROM itk_template WHERE id = ?1")
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await?;
        row.map(row_to_template).transpose()
    }

    async fn itk_template_get_for_crop(&self, crop_id: CropId) -> DbResult<Option<ItkTemplate>> {
        let row = sqlx::query("SELECT id, crop_id FROM itk_template WHERE crop_id = ?1")
            .bind(crop_id.as_uuid())
            .fetch_optional(&self.pool)
            .await?;
        row.map(row_to_template).transpose()
    }

    async fn itk_template_create(&self, template: &ItkTemplate) -> DbResult<()> {
        sqlx::query("INSERT INTO itk_template (id, crop_id) VALUES (?1, ?2)")
            .bind(template.id.as_uuid())
            .bind(template.crop_id.as_uuid())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn itk_template_delete(&self, id: ItkTemplateId) -> DbResult<()> {
        let result = sqlx::query("DELETE FROM itk_template WHERE id = ?1")
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(DbError::NotFound {
                kind: "itk_template",
                id: id.to_string(),
            });
        }
        Ok(())
    }

    async fn itk_activity_list_for_template(
        &self,
        template_id: ItkTemplateId,
    ) -> DbResult<Vec<ItkActivity>> {
        let rows = sqlx::query(&format!(
            "SELECT {ACTIVITY_COLUMNS} FROM itk_activity \
             WHERE template_id = ?1 ORDER BY position, id"
        ))
        .bind(template_id.as_uuid())
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_activity).collect()
    }

    async fn itk_activity_create(&self, a: &ItkActivity) -> DbResult<()> {
        sqlx::query(&format!(
            "INSERT INTO itk_activity ({ACTIVITY_COLUMNS}) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"
        ))
        .bind(a.id.as_uuid())
        .bind(a.template_id.as_uuid())
        .bind(a.task_type_id.as_uuid())
        .bind(i64::from(a.offset_days))
        .bind(a.method_id.map(TaskMethodId::as_uuid))
        .bind(a.implement_id.map(TaskImplementId::as_uuid))
        .bind(a.label.as_deref())
        .bind(i64::from(a.position))
        .bind(a.notes.as_deref())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn itk_activity_update(&self, a: &ItkActivity) -> DbResult<()> {
        let result = sqlx::query(
            "UPDATE itk_activity SET task_type_id = ?2, offset_days = ?3, method_id = ?4, \
             implement_id = ?5, label = ?6, position = ?7, notes = ?8 WHERE id = ?1",
        )
        .bind(a.id.as_uuid())
        .bind(a.task_type_id.as_uuid())
        .bind(i64::from(a.offset_days))
        .bind(a.method_id.map(TaskMethodId::as_uuid))
        .bind(a.implement_id.map(TaskImplementId::as_uuid))
        .bind(a.label.as_deref())
        .bind(i64::from(a.position))
        .bind(a.notes.as_deref())
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(DbError::NotFound {
                kind: "itk_activity",
                id: a.id.to_string(),
            });
        }
        Ok(())
    }

    async fn itk_activity_delete(&self, id: ItkActivityId) -> DbResult<()> {
        let result = sqlx::query("DELETE FROM itk_activity WHERE id = ?1")
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(DbError::NotFound {
                kind: "itk_activity",
                id: id.to_string(),
            });
        }
        Ok(())
    }
}

fn row_to_template(row: sqlx::sqlite::SqliteRow) -> DbResult<ItkTemplate> {
    let id: Uuid = row.try_get("id")?;
    let crop_id: Uuid = row.try_get("crop_id")?;
    Ok(ItkTemplate {
        id: ItkTemplateId::from(id),
        crop_id: CropId::from(crop_id),
    })
}

fn row_to_activity(row: sqlx::sqlite::SqliteRow) -> DbResult<ItkActivity> {
    let id: Uuid = row.try_get("id")?;
    let template_id: Uuid = row.try_get("template_id")?;
    let task_type_id: Uuid = row.try_get("task_type_id")?;
    let offset_days: i64 = row.try_get("offset_days")?;
    let position: i64 = row.try_get("position")?;
    let method_id: Option<Uuid> = row.try_get("method_id")?;
    let implement_id: Option<Uuid> = row.try_get("implement_id")?;
    Ok(ItkActivity {
        id: ItkActivityId::from(id),
        template_id: ItkTemplateId::from(template_id),
        task_type_id: TaskTypeId::from(task_type_id),
        offset_days: i32::try_from(offset_days).map_err(|_| {
            DbError::Malformed(format!("offset_days out of i32 range: {offset_days}"))
        })?,
        method_id: method_id.map(TaskMethodId::from),
        implement_id: implement_id.map(TaskImplementId::from),
        label: row.try_get("label")?,
        position: u32::try_from(position)
            .map_err(|_| DbError::Malformed(format!("position out of u32 range: {position}")))?,
        notes: row.try_get("notes")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::{CropRepo, FamilyRepo, TaskTypeRepo};
    use pomone_domain::{Crop, Family, Lifespan, PruningSeason, TaskCategory, TaskType};

    async fn repo_with_crop_and_type() -> (SqliteRepository, CropId, TaskTypeId) {
        let repo = SqliteRepository::in_memory().await.unwrap();
        let f = Family::new("Asteraceae", None, None).unwrap();
        repo.family_create(&f).await.unwrap();
        let crop = Crop::new(f.id, "Laitue", None, Lifespan::Annual, PruningSeason::None).unwrap();
        repo.crop_create(&crop).await.unwrap();
        let tt = TaskType::new("Semis", TaskCategory::Sow, "#3C6E47").unwrap();
        repo.task_type_create(&tt).await.unwrap();
        (repo, crop.id, tt.id)
    }

    #[tokio::test]
    async fn template_and_activities_roundtrip_ordered_by_position() {
        let (repo, crop, tt) = repo_with_crop_and_type().await;
        let template = ItkTemplate::new(crop);
        repo.itk_template_create(&template).await.unwrap();
        assert_eq!(
            repo.itk_template_get_for_crop(crop).await.unwrap(),
            Some(template.clone())
        );

        // Insert out of position order; the list must come back sorted.
        let a1 = ItkActivity::new(
            template.id,
            tt,
            20,
            None,
            None,
            Some("désherbage".into()),
            1,
            None,
        );
        let a0 = ItkActivity::new(
            template.id,
            tt,
            -10,
            None,
            None,
            Some("prépa".into()),
            0,
            None,
        );
        repo.itk_activity_create(&a1).await.unwrap();
        repo.itk_activity_create(&a0).await.unwrap();
        let got = repo
            .itk_activity_list_for_template(template.id)
            .await
            .unwrap();
        assert_eq!(got, vec![a0.clone(), a1.clone()]);
        assert_eq!(got[0].offset_days, -10);
    }

    #[tokio::test]
    async fn one_template_per_crop_is_enforced() {
        let (repo, crop, _tt) = repo_with_crop_and_type().await;
        repo.itk_template_create(&ItkTemplate::new(crop))
            .await
            .unwrap();
        // A second template for the same crop violates the UNIQUE(crop_id).
        assert!(repo
            .itk_template_create(&ItkTemplate::new(crop))
            .await
            .is_err());
    }

    #[tokio::test]
    async fn deleting_template_cascades_to_activities() {
        let (repo, crop, tt) = repo_with_crop_and_type().await;
        let template = ItkTemplate::new(crop);
        repo.itk_template_create(&template).await.unwrap();
        let a = ItkActivity::new(template.id, tt, 0, None, None, None, 0, None);
        repo.itk_activity_create(&a).await.unwrap();
        repo.itk_template_delete(template.id).await.unwrap();
        assert!(repo
            .itk_activity_list_for_template(template.id)
            .await
            .unwrap()
            .is_empty());
    }

    #[tokio::test]
    async fn activity_update_and_delete_report_not_found() {
        let (repo, crop, tt) = repo_with_crop_and_type().await;
        let template = ItkTemplate::new(crop);
        repo.itk_template_create(&template).await.unwrap();
        let a = ItkActivity::new(template.id, tt, 0, None, None, None, 0, None);
        repo.itk_activity_create(&a).await.unwrap();

        let edited = a
            .clone()
            .with_updates(tt, 5, None, None, Some("maj".into()), 2, None);
        repo.itk_activity_update(&edited).await.unwrap();
        let got = repo
            .itk_activity_list_for_template(template.id)
            .await
            .unwrap();
        assert_eq!(got[0].offset_days, 5);
        assert_eq!(got[0].position, 2);

        repo.itk_activity_delete(a.id).await.unwrap();
        let err = repo.itk_activity_delete(a.id).await.unwrap_err();
        assert!(matches!(
            err,
            DbError::NotFound {
                kind: "itk_activity",
                ..
            }
        ));
        let err = repo.itk_activity_update(&edited).await.unwrap_err();
        assert!(matches!(
            err,
            DbError::NotFound {
                kind: "itk_activity",
                ..
            }
        ));
    }

    #[tokio::test]
    async fn deleting_crop_cascades_to_template() {
        let (repo, crop, _tt) = repo_with_crop_and_type().await;
        let template = ItkTemplate::new(crop);
        repo.itk_template_create(&template).await.unwrap();
        repo.crop_delete(crop).await.unwrap();
        assert!(repo.itk_template_get(template.id).await.unwrap().is_none());
    }
}
