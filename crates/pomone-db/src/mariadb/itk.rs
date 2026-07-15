//! `ItkRepo` implementation for MariaDB.

use crate::error::{DbError, DbResult};
use crate::mariadb::MariaDbRepository;
use crate::repository::ItkRepo;
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
impl ItkRepo for MariaDbRepository {
    async fn itk_template_get(&self, id: ItkTemplateId) -> DbResult<Option<ItkTemplate>> {
        let row = sqlx::query("SELECT id, crop_id FROM itk_template WHERE id = ?")
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await?;
        row.map(row_to_template).transpose()
    }

    async fn itk_template_get_for_crop(&self, crop_id: CropId) -> DbResult<Option<ItkTemplate>> {
        let row = sqlx::query("SELECT id, crop_id FROM itk_template WHERE crop_id = ?")
            .bind(crop_id.as_uuid())
            .fetch_optional(&self.pool)
            .await?;
        row.map(row_to_template).transpose()
    }

    async fn itk_template_create(&self, template: &ItkTemplate) -> DbResult<()> {
        sqlx::query("INSERT INTO itk_template (id, crop_id) VALUES (?, ?)")
            .bind(template.id.as_uuid())
            .bind(template.crop_id.as_uuid())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn itk_template_delete(&self, id: ItkTemplateId) -> DbResult<()> {
        let result = sqlx::query("DELETE FROM itk_template WHERE id = ?")
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
             WHERE template_id = ? ORDER BY position, id"
        ))
        .bind(template_id.as_uuid())
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_activity).collect()
    }

    async fn itk_activity_create(&self, a: &ItkActivity) -> DbResult<()> {
        sqlx::query(&format!(
            "INSERT INTO itk_activity ({ACTIVITY_COLUMNS}) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
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
            "UPDATE itk_activity SET task_type_id = ?, offset_days = ?, method_id = ?, \
             implement_id = ?, label = ?, position = ?, notes = ? WHERE id = ?",
        )
        .bind(a.task_type_id.as_uuid())
        .bind(i64::from(a.offset_days))
        .bind(a.method_id.map(TaskMethodId::as_uuid))
        .bind(a.implement_id.map(TaskImplementId::as_uuid))
        .bind(a.label.as_deref())
        .bind(i64::from(a.position))
        .bind(a.notes.as_deref())
        .bind(a.id.as_uuid())
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
        let result = sqlx::query("DELETE FROM itk_activity WHERE id = ?")
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

fn row_to_template(row: sqlx::mysql::MySqlRow) -> DbResult<ItkTemplate> {
    let id: Uuid = row.try_get("id")?;
    let crop_id: Uuid = row.try_get("crop_id")?;
    Ok(ItkTemplate {
        id: ItkTemplateId::from(id),
        crop_id: CropId::from(crop_id),
    })
}

fn row_to_activity(row: sqlx::mysql::MySqlRow) -> DbResult<ItkActivity> {
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
