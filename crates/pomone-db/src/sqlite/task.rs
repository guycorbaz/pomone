//! `TaskTypeRepo` / `TaskMethodRepo` / `TaskImplementRepo` / `TaskRepo`
//! implementations for SQLite.
//!
//! All four sub-traits share this file because they map to four closely
//! related tables (`task_type`, `task_method`, `task_implement`, `task`)
//! and the helper conversions are the same. Split them into separate
//! files if/when they grow noticeably.

use crate::codec::{
    opt_decimal_from_text, opt_decimal_to_text, task_category_from_str, task_category_to_str,
};
use crate::error::{DbError, DbResult};
use crate::repository::{TaskImplementRepo, TaskMethodRepo, TaskRepo, TaskTypeRepo};
use crate::sqlite::SqliteRepository;
use async_trait::async_trait;
use chrono::NaiveDate;
use pomone_domain::{
    LocationId, PlantingId, Task, TaskId, TaskImplement, TaskImplementId, TaskMethod, TaskMethodId,
    TaskType, TaskTypeId,
};
use sqlx::Row;
use uuid::Uuid;

// ============================================================
// TaskTypeRepo
// ============================================================

#[async_trait]
impl TaskTypeRepo for SqliteRepository {
    async fn task_type_get(&self, id: TaskTypeId) -> DbResult<Option<TaskType>> {
        let row = sqlx::query("SELECT id, name, category, color FROM task_type WHERE id = ?1")
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await?;
        row.map(row_to_task_type).transpose()
    }

    async fn task_type_list(&self) -> DbResult<Vec<TaskType>> {
        let rows = sqlx::query("SELECT id, name, category, color FROM task_type ORDER BY name")
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(row_to_task_type).collect()
    }

    async fn task_type_create(&self, t: &TaskType) -> DbResult<()> {
        sqlx::query("INSERT INTO task_type (id, name, category, color) VALUES (?1, ?2, ?3, ?4)")
            .bind(t.id.as_uuid())
            .bind(&t.name)
            .bind(task_category_to_str(t.category))
            .bind(&t.color)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn task_type_update(&self, t: &TaskType) -> DbResult<()> {
        let res =
            sqlx::query("UPDATE task_type SET name = ?2, category = ?3, color = ?4 WHERE id = ?1")
                .bind(t.id.as_uuid())
                .bind(&t.name)
                .bind(task_category_to_str(t.category))
                .bind(&t.color)
                .execute(&self.pool)
                .await?;
        if res.rows_affected() == 0 {
            return Err(DbError::NotFound {
                kind: "task_type",
                id: t.id.to_string(),
            });
        }
        Ok(())
    }

    async fn task_type_delete(&self, id: TaskTypeId) -> DbResult<()> {
        let res = sqlx::query("DELETE FROM task_type WHERE id = ?1")
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await?;
        if res.rows_affected() == 0 {
            return Err(DbError::NotFound {
                kind: "task_type",
                id: id.to_string(),
            });
        }
        Ok(())
    }
}

fn row_to_task_type(row: sqlx::sqlite::SqliteRow) -> DbResult<TaskType> {
    let id: Uuid = row.try_get("id")?;
    let category: String = row.try_get("category")?;
    Ok(TaskType {
        id: TaskTypeId::from(id),
        name: row.try_get("name")?,
        category: task_category_from_str(&category)?,
        color: row.try_get("color")?,
    })
}

// ============================================================
// TaskMethodRepo
// ============================================================

#[async_trait]
impl TaskMethodRepo for SqliteRepository {
    async fn task_method_get(&self, id: TaskMethodId) -> DbResult<Option<TaskMethod>> {
        let row = sqlx::query("SELECT id, name, notes FROM task_method WHERE id = ?1")
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await?;
        row.map(row_to_task_method).transpose()
    }

    async fn task_method_list(&self) -> DbResult<Vec<TaskMethod>> {
        let rows = sqlx::query("SELECT id, name, notes FROM task_method ORDER BY name")
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(row_to_task_method).collect()
    }

    async fn task_method_create(&self, m: &TaskMethod) -> DbResult<()> {
        sqlx::query("INSERT INTO task_method (id, name, notes) VALUES (?1, ?2, ?3)")
            .bind(m.id.as_uuid())
            .bind(&m.name)
            .bind(m.notes.as_deref())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn task_method_update(&self, m: &TaskMethod) -> DbResult<()> {
        let res = sqlx::query("UPDATE task_method SET name = ?2, notes = ?3 WHERE id = ?1")
            .bind(m.id.as_uuid())
            .bind(&m.name)
            .bind(m.notes.as_deref())
            .execute(&self.pool)
            .await?;
        if res.rows_affected() == 0 {
            return Err(DbError::NotFound {
                kind: "task_method",
                id: m.id.to_string(),
            });
        }
        Ok(())
    }

    async fn task_method_delete(&self, id: TaskMethodId) -> DbResult<()> {
        let res = sqlx::query("DELETE FROM task_method WHERE id = ?1")
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await?;
        if res.rows_affected() == 0 {
            return Err(DbError::NotFound {
                kind: "task_method",
                id: id.to_string(),
            });
        }
        Ok(())
    }
}

fn row_to_task_method(row: sqlx::sqlite::SqliteRow) -> DbResult<TaskMethod> {
    let id: Uuid = row.try_get("id")?;
    Ok(TaskMethod {
        id: TaskMethodId::from(id),
        name: row.try_get("name")?,
        notes: row.try_get("notes")?,
    })
}

// ============================================================
// TaskImplementRepo
// ============================================================

#[async_trait]
impl TaskImplementRepo for SqliteRepository {
    async fn task_implement_get(&self, id: TaskImplementId) -> DbResult<Option<TaskImplement>> {
        let row = sqlx::query("SELECT id, name, notes FROM task_implement WHERE id = ?1")
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await?;
        row.map(row_to_task_implement).transpose()
    }

    async fn task_implement_list(&self) -> DbResult<Vec<TaskImplement>> {
        let rows = sqlx::query("SELECT id, name, notes FROM task_implement ORDER BY name")
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(row_to_task_implement).collect()
    }

    async fn task_implement_create(&self, i: &TaskImplement) -> DbResult<()> {
        sqlx::query("INSERT INTO task_implement (id, name, notes) VALUES (?1, ?2, ?3)")
            .bind(i.id.as_uuid())
            .bind(&i.name)
            .bind(i.notes.as_deref())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn task_implement_update(&self, i: &TaskImplement) -> DbResult<()> {
        let res = sqlx::query("UPDATE task_implement SET name = ?2, notes = ?3 WHERE id = ?1")
            .bind(i.id.as_uuid())
            .bind(&i.name)
            .bind(i.notes.as_deref())
            .execute(&self.pool)
            .await?;
        if res.rows_affected() == 0 {
            return Err(DbError::NotFound {
                kind: "task_implement",
                id: i.id.to_string(),
            });
        }
        Ok(())
    }

    async fn task_implement_delete(&self, id: TaskImplementId) -> DbResult<()> {
        let res = sqlx::query("DELETE FROM task_implement WHERE id = ?1")
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await?;
        if res.rows_affected() == 0 {
            return Err(DbError::NotFound {
                kind: "task_implement",
                id: id.to_string(),
            });
        }
        Ok(())
    }
}

fn row_to_task_implement(row: sqlx::sqlite::SqliteRow) -> DbResult<TaskImplement> {
    let id: Uuid = row.try_get("id")?;
    Ok(TaskImplement {
        id: TaskImplementId::from(id),
        name: row.try_get("name")?,
        notes: row.try_get("notes")?,
    })
}

// ============================================================
// TaskRepo
// ============================================================

const TASK_COLUMNS: &str = "id, planting_id, location_id, task_type_id, task_method_id, \
                            implement_id, planned_on, completed_on, duration_min, labor_hours, notes";

#[async_trait]
impl TaskRepo for SqliteRepository {
    async fn task_get(&self, id: TaskId) -> DbResult<Option<Task>> {
        let row = sqlx::query(&format!("SELECT {TASK_COLUMNS} FROM task WHERE id = ?1"))
            .bind(id.as_uuid())
            .fetch_optional(&self.pool)
            .await?;
        row.map(row_to_task).transpose()
    }

    async fn task_list(&self) -> DbResult<Vec<Task>> {
        let rows = sqlx::query(&format!(
            "SELECT {TASK_COLUMNS} FROM task ORDER BY planned_on"
        ))
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_task).collect()
    }

    async fn task_list_for_planting(&self, planting_id: PlantingId) -> DbResult<Vec<Task>> {
        let rows = sqlx::query(&format!(
            "SELECT {TASK_COLUMNS} FROM task WHERE planting_id = ?1 ORDER BY planned_on"
        ))
        .bind(planting_id.as_uuid())
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_task).collect()
    }

    async fn task_list_for_location(&self, location_id: LocationId) -> DbResult<Vec<Task>> {
        let rows = sqlx::query(&format!(
            "SELECT {TASK_COLUMNS} FROM task WHERE location_id = ?1 ORDER BY planned_on"
        ))
        .bind(location_id.as_uuid())
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_task).collect()
    }

    async fn task_list_in_range(&self, from: NaiveDate, to: NaiveDate) -> DbResult<Vec<Task>> {
        let rows = sqlx::query(&format!(
            "SELECT {TASK_COLUMNS} FROM task \
             WHERE planned_on BETWEEN ?1 AND ?2 ORDER BY planned_on"
        ))
        .bind(from)
        .bind(to)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_task).collect()
    }

    async fn task_create(&self, t: &Task) -> DbResult<()> {
        sqlx::query(&format!(
            "INSERT INTO task ({TASK_COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)"
        ))
        .bind(t.id.as_uuid())
        .bind(t.planting_id.map(PlantingId::as_uuid))
        .bind(t.location_id.map(LocationId::as_uuid))
        .bind(t.task_type_id.as_uuid())
        .bind(t.task_method_id.map(TaskMethodId::as_uuid))
        .bind(t.implement_id.map(TaskImplementId::as_uuid))
        .bind(t.planned_on)
        .bind(t.completed_on)
        .bind(t.duration_min.map(i64::from))
        .bind(opt_decimal_to_text(t.labor_hours))
        .bind(t.notes.as_deref())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn task_update(&self, t: &Task) -> DbResult<()> {
        let res = sqlx::query(
            "UPDATE task SET planting_id = ?2, location_id = ?3, task_type_id = ?4, \
             task_method_id = ?5, implement_id = ?6, planned_on = ?7, completed_on = ?8, \
             duration_min = ?9, labor_hours = ?10, notes = ?11 WHERE id = ?1",
        )
        .bind(t.id.as_uuid())
        .bind(t.planting_id.map(PlantingId::as_uuid))
        .bind(t.location_id.map(LocationId::as_uuid))
        .bind(t.task_type_id.as_uuid())
        .bind(t.task_method_id.map(TaskMethodId::as_uuid))
        .bind(t.implement_id.map(TaskImplementId::as_uuid))
        .bind(t.planned_on)
        .bind(t.completed_on)
        .bind(t.duration_min.map(i64::from))
        .bind(opt_decimal_to_text(t.labor_hours))
        .bind(t.notes.as_deref())
        .execute(&self.pool)
        .await?;
        if res.rows_affected() == 0 {
            return Err(DbError::NotFound {
                kind: "task",
                id: t.id.to_string(),
            });
        }
        Ok(())
    }

    async fn task_delete(&self, id: TaskId) -> DbResult<()> {
        let res = sqlx::query("DELETE FROM task WHERE id = ?1")
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await?;
        if res.rows_affected() == 0 {
            return Err(DbError::NotFound {
                kind: "task",
                id: id.to_string(),
            });
        }
        Ok(())
    }
}

fn row_to_task(row: sqlx::sqlite::SqliteRow) -> DbResult<Task> {
    let id: Uuid = row.try_get("id")?;
    let task_type_id: Uuid = row.try_get("task_type_id")?;
    let planting_id: Option<Uuid> = row.try_get("planting_id")?;
    let location_id: Option<Uuid> = row.try_get("location_id")?;
    let task_method_id: Option<Uuid> = row.try_get("task_method_id")?;
    let implement_id: Option<Uuid> = row.try_get("implement_id")?;
    let duration_min: Option<i64> = row.try_get("duration_min")?;
    let labor_hours_text: Option<String> = row.try_get("labor_hours")?;
    let duration_min = duration_min
        .map(|v| {
            u32::try_from(v)
                .map_err(|_| DbError::Malformed(format!("duration_min out of u32 range: {v}")))
        })
        .transpose()?;
    Ok(Task {
        id: TaskId::from(id),
        planting_id: planting_id.map(PlantingId::from),
        location_id: location_id.map(LocationId::from),
        task_type_id: TaskTypeId::from(task_type_id),
        task_method_id: task_method_id.map(TaskMethodId::from),
        implement_id: implement_id.map(TaskImplementId::from),
        planned_on: row.try_get("planned_on")?,
        completed_on: row.try_get("completed_on")?,
        duration_min,
        labor_hours: opt_decimal_from_text(labor_hours_text)?,
        notes: row.try_get("notes")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use pomone_domain::TaskCategory;
    use rust_decimal_macros::dec;

    async fn fresh_repo_with_type() -> (SqliteRepository, TaskTypeId) {
        let repo = SqliteRepository::in_memory().await.unwrap();
        let tt = TaskType::new("Semis", TaskCategory::Sow, "#3C6E47").unwrap();
        let id = tt.id;
        repo.task_type_create(&tt).await.unwrap();
        (repo, id)
    }

    #[tokio::test]
    async fn task_type_crud_roundtrip() {
        let (repo, id) = fresh_repo_with_type().await;
        let got = repo.task_type_get(id).await.unwrap().unwrap();
        assert_eq!(got.name, "Semis");
        assert_eq!(got.category, TaskCategory::Sow);

        // Rename + recolor.
        let renamed = got.rename("Semis direct", "#244529").unwrap();
        repo.task_type_update(&renamed).await.unwrap();
        let updated = repo.task_type_get(id).await.unwrap().unwrap();
        assert_eq!(updated.name, "Semis direct");
        assert_eq!(updated.color, "#244529");
        assert_eq!(updated.category, TaskCategory::Sow);

        repo.task_type_delete(id).await.unwrap();
        assert!(repo.task_type_get(id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn task_method_and_implement_crud() {
        let repo = SqliteRepository::in_memory().await.unwrap();
        let m = TaskMethod::new("Manuel", None).unwrap();
        repo.task_method_create(&m).await.unwrap();
        assert_eq!(repo.task_method_list().await.unwrap().len(), 1);

        let i = TaskImplement::new("Houe", Some("Sarclage".into())).unwrap();
        repo.task_implement_create(&i).await.unwrap();
        assert_eq!(
            repo.task_implement_get(i.id).await.unwrap().unwrap().notes,
            Some("Sarclage".to_string())
        );
    }

    #[tokio::test]
    async fn task_crud_and_filters() {
        let (repo, tt_id) = fresh_repo_with_type().await;

        let t1 = Task::new(
            None,
            None,
            tt_id,
            None,
            None,
            NaiveDate::from_ymd_opt(2026, 5, 10).unwrap(),
            None,
            Some(45),
            Some(dec!(0.75)),
            Some("first".into()),
        );
        let t2 = Task::new(
            None,
            None,
            tt_id,
            None,
            None,
            NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(),
            None,
            None,
            None,
            None,
        );
        repo.task_create(&t1).await.unwrap();
        repo.task_create(&t2).await.unwrap();

        // list returns both, ordered by planned_on
        let all = repo.task_list().await.unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].id, t1.id);

        // range filter is inclusive on both ends
        let in_may = repo
            .task_list_in_range(
                NaiveDate::from_ymd_opt(2026, 5, 1).unwrap(),
                NaiveDate::from_ymd_opt(2026, 5, 31).unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(in_may.len(), 1);
        assert_eq!(in_may[0].id, t1.id);

        // mark t1 completed, then update
        let done = t1.mark_completed(NaiveDate::from_ymd_opt(2026, 5, 11).unwrap());
        repo.task_update(&done).await.unwrap();
        let got = repo.task_get(done.id).await.unwrap().unwrap();
        assert_eq!(
            got.completed_on,
            Some(NaiveDate::from_ymd_opt(2026, 5, 11).unwrap())
        );

        // delete
        repo.task_delete(t2.id).await.unwrap();
        assert!(repo.task_get(t2.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn deleting_type_with_tasks_is_restricted() {
        let (repo, tt_id) = fresh_repo_with_type().await;
        let t = Task::new(
            None,
            None,
            tt_id,
            None,
            None,
            NaiveDate::from_ymd_opt(2026, 5, 10).unwrap(),
            None,
            None,
            None,
            None,
        );
        repo.task_create(&t).await.unwrap();
        // FK on task.task_type_id is ON DELETE RESTRICT — deletion must error.
        let err = repo.task_type_delete(tt_id).await.unwrap_err();
        assert!(matches!(err, DbError::Sqlx(_)), "got {err:?}");
    }
}
