//! Task taxonomy + main `task` table implementations for MariaDB.
//!
//! Mirrors `sqlite/task.rs`, with `?` placeholders and native `DECIMAL`
//! handling (no TEXT codec needed — sqlx-mysql supports `rust_decimal`).

use crate::codec::{
    opt_skip_reason_from_text, opt_skip_reason_to_text, recurrence_unit_from_str,
    recurrence_unit_to_str, task_category_from_str, task_category_to_str,
};
use crate::error::{DbError, DbResult};
use crate::mariadb::MariaDbRepository;
use crate::repository::{
    TaskImplementRepo, TaskMethodRepo, TaskRepo, TaskSeriesRepo, TaskTypeRepo,
};
use async_trait::async_trait;
use chrono::NaiveDate;
use pomone_domain::{
    LocationId, PlantingId, RecurrenceRule, Task, TaskId, TaskImplement, TaskImplementId,
    TaskMethod, TaskMethodId, TaskSeries, TaskSeriesId, TaskType, TaskTypeId,
};
use rust_decimal::Decimal;
use sqlx::Row;
use uuid::Uuid;

// ============================================================
// TaskTypeRepo
// ============================================================

#[async_trait]
impl TaskTypeRepo for MariaDbRepository {
    async fn task_type_get(&self, id: TaskTypeId) -> DbResult<Option<TaskType>> {
        let row = sqlx::query("SELECT id, name, category, color FROM task_type WHERE id = ?")
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
        sqlx::query("INSERT INTO task_type (id, name, category, color) VALUES (?, ?, ?, ?)")
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
            sqlx::query("UPDATE task_type SET name = ?, category = ?, color = ? WHERE id = ?")
                .bind(&t.name)
                .bind(task_category_to_str(t.category))
                .bind(&t.color)
                .bind(t.id.as_uuid())
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
        let res = sqlx::query("DELETE FROM task_type WHERE id = ?")
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

fn row_to_task_type(row: sqlx::mysql::MySqlRow) -> DbResult<TaskType> {
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
impl TaskMethodRepo for MariaDbRepository {
    async fn task_method_get(&self, id: TaskMethodId) -> DbResult<Option<TaskMethod>> {
        let row = sqlx::query("SELECT id, name, notes FROM task_method WHERE id = ?")
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
        sqlx::query("INSERT INTO task_method (id, name, notes) VALUES (?, ?, ?)")
            .bind(m.id.as_uuid())
            .bind(&m.name)
            .bind(m.notes.as_deref())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn task_method_update(&self, m: &TaskMethod) -> DbResult<()> {
        let res = sqlx::query("UPDATE task_method SET name = ?, notes = ? WHERE id = ?")
            .bind(&m.name)
            .bind(m.notes.as_deref())
            .bind(m.id.as_uuid())
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
        let res = sqlx::query("DELETE FROM task_method WHERE id = ?")
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

fn row_to_task_method(row: sqlx::mysql::MySqlRow) -> DbResult<TaskMethod> {
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
impl TaskImplementRepo for MariaDbRepository {
    async fn task_implement_get(&self, id: TaskImplementId) -> DbResult<Option<TaskImplement>> {
        let row = sqlx::query("SELECT id, name, notes FROM task_implement WHERE id = ?")
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
        sqlx::query("INSERT INTO task_implement (id, name, notes) VALUES (?, ?, ?)")
            .bind(i.id.as_uuid())
            .bind(&i.name)
            .bind(i.notes.as_deref())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn task_implement_update(&self, i: &TaskImplement) -> DbResult<()> {
        let res = sqlx::query("UPDATE task_implement SET name = ?, notes = ? WHERE id = ?")
            .bind(&i.name)
            .bind(i.notes.as_deref())
            .bind(i.id.as_uuid())
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
        let res = sqlx::query("DELETE FROM task_implement WHERE id = ?")
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

fn row_to_task_implement(row: sqlx::mysql::MySqlRow) -> DbResult<TaskImplement> {
    let id: Uuid = row.try_get("id")?;
    Ok(TaskImplement {
        id: TaskImplementId::from(id),
        name: row.try_get("name")?,
        notes: row.try_get("notes")?,
    })
}

// ============================================================
// TaskSeriesRepo
// ============================================================

const TASK_SERIES_COLUMNS: &str =
    "id, planting_id, location_id, task_type_id, task_method_id, implement_id, \
     recurrence_unit, recurrence_interval, first_planned_on, end_on, notes";

#[async_trait]
impl TaskSeriesRepo for MariaDbRepository {
    async fn task_series_get(&self, id: TaskSeriesId) -> DbResult<Option<TaskSeries>> {
        let row = sqlx::query(&format!(
            "SELECT {TASK_SERIES_COLUMNS} FROM task_series WHERE id = ?"
        ))
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await?;
        row.map(row_to_task_series).transpose()
    }

    async fn task_series_list(&self) -> DbResult<Vec<TaskSeries>> {
        let rows = sqlx::query(&format!(
            "SELECT {TASK_SERIES_COLUMNS} FROM task_series ORDER BY first_planned_on"
        ))
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_task_series).collect()
    }

    async fn task_series_create(&self, s: &TaskSeries) -> DbResult<()> {
        sqlx::query(&format!(
            "INSERT INTO task_series ({TASK_SERIES_COLUMNS}) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        ))
        .bind(s.id.as_uuid())
        .bind(s.planting_id.map(PlantingId::as_uuid))
        .bind(s.location_id.map(LocationId::as_uuid))
        .bind(s.task_type_id.as_uuid())
        .bind(s.task_method_id.map(TaskMethodId::as_uuid))
        .bind(s.implement_id.map(TaskImplementId::as_uuid))
        .bind(recurrence_unit_to_str(s.rule.unit))
        .bind(i64::from(s.rule.interval))
        .bind(s.first_planned_on)
        .bind(s.rule.end_on)
        .bind(s.notes.as_deref())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn task_series_delete(&self, id: TaskSeriesId) -> DbResult<()> {
        let res = sqlx::query("DELETE FROM task_series WHERE id = ?")
            .bind(id.as_uuid())
            .execute(&self.pool)
            .await?;
        if res.rows_affected() == 0 {
            return Err(DbError::NotFound {
                kind: "task_series",
                id: id.to_string(),
            });
        }
        Ok(())
    }
}

fn row_to_task_series(row: sqlx::mysql::MySqlRow) -> DbResult<TaskSeries> {
    let id: Uuid = row.try_get("id")?;
    let task_type_id: Uuid = row.try_get("task_type_id")?;
    let planting_id: Option<Uuid> = row.try_get("planting_id")?;
    let location_id: Option<Uuid> = row.try_get("location_id")?;
    let task_method_id: Option<Uuid> = row.try_get("task_method_id")?;
    let implement_id: Option<Uuid> = row.try_get("implement_id")?;
    let unit_str: String = row.try_get("recurrence_unit")?;
    let interval_raw: i32 = row.try_get("recurrence_interval")?;
    let interval = u32::try_from(interval_raw).map_err(|_| {
        DbError::Malformed(format!(
            "recurrence_interval out of u32 range: {interval_raw}"
        ))
    })?;
    let rule = RecurrenceRule::new(
        recurrence_unit_from_str(&unit_str)?,
        interval,
        row.try_get("end_on")?,
    )
    .map_err(|e| DbError::Malformed(format!("bad recurrence rule: {e}")))?;
    Ok(TaskSeries {
        id: TaskSeriesId::from(id),
        planting_id: planting_id.map(PlantingId::from),
        location_id: location_id.map(LocationId::from),
        task_type_id: TaskTypeId::from(task_type_id),
        task_method_id: task_method_id.map(TaskMethodId::from),
        implement_id: implement_id.map(TaskImplementId::from),
        rule,
        first_planned_on: row.try_get("first_planned_on")?,
        notes: row.try_get("notes")?,
    })
}

// ============================================================
// TaskRepo
// ============================================================

const TASK_COLUMNS: &str = "id, planting_id, location_id, task_type_id, task_method_id, \
                            implement_id, series_id, planned_on, completed_on, duration_min, \
                            labor_hours, notes, skipped_on, skip_reason, skip_note";

#[async_trait]
impl TaskRepo for MariaDbRepository {
    async fn task_get(&self, id: TaskId) -> DbResult<Option<Task>> {
        let row = sqlx::query(&format!("SELECT {TASK_COLUMNS} FROM task WHERE id = ?"))
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
            "SELECT {TASK_COLUMNS} FROM task WHERE planting_id = ? ORDER BY planned_on"
        ))
        .bind(planting_id.as_uuid())
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_task).collect()
    }

    async fn task_list_for_location(&self, location_id: LocationId) -> DbResult<Vec<Task>> {
        let rows = sqlx::query(&format!(
            "SELECT {TASK_COLUMNS} FROM task WHERE location_id = ? ORDER BY planned_on"
        ))
        .bind(location_id.as_uuid())
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_task).collect()
    }

    async fn task_list_in_range(&self, from: NaiveDate, to: NaiveDate) -> DbResult<Vec<Task>> {
        let rows = sqlx::query(&format!(
            "SELECT {TASK_COLUMNS} FROM task \
             WHERE planned_on BETWEEN ? AND ? ORDER BY planned_on"
        ))
        .bind(from)
        .bind(to)
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter().map(row_to_task).collect()
    }

    async fn task_create(&self, t: &Task) -> DbResult<()> {
        // Skip columns are part of INSERT for completeness (NULL on a fresh
        // task); their projection is written only by `facts::record_fact`
        // (story 1.2), never here.
        sqlx::query(&format!(
            "INSERT INTO task ({TASK_COLUMNS}) VALUES \
             (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        ))
        .bind(t.id.as_uuid())
        .bind(t.planting_id.map(PlantingId::as_uuid))
        .bind(t.location_id.map(LocationId::as_uuid))
        .bind(t.task_type_id.as_uuid())
        .bind(t.task_method_id.map(TaskMethodId::as_uuid))
        .bind(t.implement_id.map(TaskImplementId::as_uuid))
        .bind(t.series_id.map(TaskSeriesId::as_uuid))
        .bind(t.planned_on)
        .bind(t.completed_on)
        .bind(t.duration_min.map(i64::from))
        .bind(t.labor_hours)
        .bind(t.notes.as_deref())
        .bind(t.skipped_on)
        .bind(opt_skip_reason_to_text(t.skip_reason))
        .bind(t.skip_note.as_deref())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn task_update(&self, t: &Task) -> DbResult<()> {
        // Settled-state columns are projected exclusively by
        // `facts::record_fact` (story 1.2) — never written here (lint-enforced).
        let res = sqlx::query(
            "UPDATE task SET planting_id = ?, location_id = ?, task_type_id = ?, \
             task_method_id = ?, implement_id = ?, series_id = ?, planned_on = ?, \
             duration_min = ?, labor_hours = ?, notes = ? WHERE id = ?",
        )
        .bind(t.planting_id.map(PlantingId::as_uuid))
        .bind(t.location_id.map(LocationId::as_uuid))
        .bind(t.task_type_id.as_uuid())
        .bind(t.task_method_id.map(TaskMethodId::as_uuid))
        .bind(t.implement_id.map(TaskImplementId::as_uuid))
        .bind(t.series_id.map(TaskSeriesId::as_uuid))
        .bind(t.planned_on)
        .bind(t.duration_min.map(i64::from))
        .bind(t.labor_hours)
        .bind(t.notes.as_deref())
        .bind(t.id.as_uuid())
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
        let res = sqlx::query("DELETE FROM task WHERE id = ?")
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

fn row_to_task(row: sqlx::mysql::MySqlRow) -> DbResult<Task> {
    let id: Uuid = row.try_get("id")?;
    let task_type_id: Uuid = row.try_get("task_type_id")?;
    let planting_id: Option<Uuid> = row.try_get("planting_id")?;
    let location_id: Option<Uuid> = row.try_get("location_id")?;
    let task_method_id: Option<Uuid> = row.try_get("task_method_id")?;
    let implement_id: Option<Uuid> = row.try_get("implement_id")?;
    let series_id: Option<Uuid> = row.try_get("series_id")?;
    let duration_min: Option<i64> = row.try_get("duration_min")?;
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
        series_id: series_id.map(TaskSeriesId::from),
        planned_on: row.try_get("planned_on")?,
        completed_on: row.try_get("completed_on")?,
        duration_min,
        labor_hours: row.try_get::<Option<Decimal>, _>("labor_hours")?,
        notes: row.try_get("notes")?,
        skipped_on: row.try_get("skipped_on")?,
        skip_reason: opt_skip_reason_from_text(row.try_get("skip_reason")?)?,
        skip_note: row.try_get("skip_note")?,
    })
}
