//! Backend-agnostic repository traits.
//!
//! Each entity has its own sub-trait (`FamilyRepo`, `CropRepo`, …). The
//! aggregated [`Repository`] trait is what application code depends on; the
//! concrete backends (`pomone_db::sqlite::SqliteRepository`, future MariaDB)
//! implement all sub-traits and are erased behind `dyn Repository`.

use crate::error::DbResult;
use async_trait::async_trait;
use pomone_domain::{
    Crop, CropId, Family, FamilyId, FieldEvent, FieldEventId, Location, LocationId, LocationKind,
    LocationKindId, Planting, PlantingId, Strata, StrataId, Task, TaskId, TaskImplement,
    TaskImplementId, TaskMethod, TaskMethodId, TaskSeries, TaskSeriesId, TaskType, TaskTypeId,
    Treatment, TreatmentId, Variety, VarietyId, YearlyHarvest,
};
use uuid::Uuid;

#[async_trait]
pub trait FamilyRepo: Send + Sync {
    async fn family_get(&self, id: FamilyId) -> DbResult<Option<Family>>;
    async fn family_list(&self) -> DbResult<Vec<Family>>;
    async fn family_create(&self, family: &Family) -> DbResult<()>;
    async fn family_update(&self, family: &Family) -> DbResult<()>;
    async fn family_delete(&self, id: FamilyId) -> DbResult<()>;
}

#[async_trait]
pub trait StrataRepo: Send + Sync {
    async fn strata_get(&self, id: StrataId) -> DbResult<Option<Strata>>;
    async fn strata_list(&self) -> DbResult<Vec<Strata>>;
    async fn strata_create(&self, strata: &Strata) -> DbResult<()>;
    async fn strata_update(&self, strata: &Strata) -> DbResult<()>;
    async fn strata_delete(&self, id: StrataId) -> DbResult<()>;
}

#[async_trait]
pub trait LocationKindRepo: Send + Sync {
    async fn location_kind_get(&self, id: LocationKindId) -> DbResult<Option<LocationKind>>;
    async fn location_kind_list(&self) -> DbResult<Vec<LocationKind>>;
    async fn location_kind_create(&self, kind: &LocationKind) -> DbResult<()>;
    async fn location_kind_update(&self, kind: &LocationKind) -> DbResult<()>;
    async fn location_kind_delete(&self, id: LocationKindId) -> DbResult<()>;
}

#[async_trait]
pub trait LocationRepo: Send + Sync {
    async fn location_get(&self, id: LocationId) -> DbResult<Option<Location>>;
    async fn location_list(&self) -> DbResult<Vec<Location>>;
    async fn location_list_roots(&self) -> DbResult<Vec<Location>>;
    async fn location_list_children(&self, parent_id: LocationId) -> DbResult<Vec<Location>>;
    async fn location_create(&self, location: &Location) -> DbResult<()>;
    async fn location_update(&self, location: &Location) -> DbResult<()>;
    async fn location_delete(&self, id: LocationId) -> DbResult<()>;
}

#[async_trait]
pub trait CropRepo: Send + Sync {
    async fn crop_get(&self, id: CropId) -> DbResult<Option<Crop>>;
    async fn crop_list(&self) -> DbResult<Vec<Crop>>;
    async fn crop_create(&self, crop: &Crop) -> DbResult<()>;
    async fn crop_update(&self, crop: &Crop) -> DbResult<()>;
    async fn crop_delete(&self, id: CropId) -> DbResult<()>;
}

#[async_trait]
pub trait VarietyRepo: Send + Sync {
    async fn variety_get(&self, id: VarietyId) -> DbResult<Option<Variety>>;
    async fn variety_list(&self) -> DbResult<Vec<Variety>>;
    async fn variety_list_for_crop(&self, crop_id: CropId) -> DbResult<Vec<Variety>>;
    async fn variety_create(&self, variety: &Variety) -> DbResult<()>;
    async fn variety_update(&self, variety: &Variety) -> DbResult<()>;
    async fn variety_delete(&self, id: VarietyId) -> DbResult<()>;
}

#[async_trait]
pub trait PlantingRepo: Send + Sync {
    async fn planting_get(&self, id: PlantingId) -> DbResult<Option<Planting>>;
    async fn planting_list(&self) -> DbResult<Vec<Planting>>;
    async fn planting_list_for_location(&self, location_id: LocationId) -> DbResult<Vec<Planting>>;
    async fn planting_create(&self, planting: &Planting) -> DbResult<()>;
    async fn planting_update(&self, planting: &Planting) -> DbResult<()>;
    async fn planting_delete(&self, id: PlantingId) -> DbResult<()>;
}

#[async_trait]
pub trait YearlyHarvestRepo: Send + Sync {
    async fn yearly_harvest_get(
        &self,
        planting_id: PlantingId,
        year: i32,
    ) -> DbResult<Option<YearlyHarvest>>;
    async fn yearly_harvest_list_for_planting(
        &self,
        planting_id: PlantingId,
    ) -> DbResult<Vec<YearlyHarvest>>;
    async fn yearly_harvest_upsert(&self, harvest: &YearlyHarvest) -> DbResult<()>;
    async fn yearly_harvest_delete(&self, planting_id: PlantingId, year: i32) -> DbResult<()>;
}

#[async_trait]
pub trait TaskTypeRepo: Send + Sync {
    async fn task_type_get(&self, id: TaskTypeId) -> DbResult<Option<TaskType>>;
    async fn task_type_list(&self) -> DbResult<Vec<TaskType>>;
    async fn task_type_create(&self, task_type: &TaskType) -> DbResult<()>;
    async fn task_type_update(&self, task_type: &TaskType) -> DbResult<()>;
    async fn task_type_delete(&self, id: TaskTypeId) -> DbResult<()>;
}

#[async_trait]
pub trait TaskMethodRepo: Send + Sync {
    async fn task_method_get(&self, id: TaskMethodId) -> DbResult<Option<TaskMethod>>;
    async fn task_method_list(&self) -> DbResult<Vec<TaskMethod>>;
    async fn task_method_create(&self, method: &TaskMethod) -> DbResult<()>;
    async fn task_method_update(&self, method: &TaskMethod) -> DbResult<()>;
    async fn task_method_delete(&self, id: TaskMethodId) -> DbResult<()>;
}

#[async_trait]
pub trait TaskImplementRepo: Send + Sync {
    async fn task_implement_get(&self, id: TaskImplementId) -> DbResult<Option<TaskImplement>>;
    async fn task_implement_list(&self) -> DbResult<Vec<TaskImplement>>;
    async fn task_implement_create(&self, implement: &TaskImplement) -> DbResult<()>;
    async fn task_implement_update(&self, implement: &TaskImplement) -> DbResult<()>;
    async fn task_implement_delete(&self, id: TaskImplementId) -> DbResult<()>;
}

#[async_trait]
pub trait TaskSeriesRepo: Send + Sync {
    async fn task_series_get(&self, id: TaskSeriesId) -> DbResult<Option<TaskSeries>>;
    async fn task_series_list(&self) -> DbResult<Vec<TaskSeries>>;
    async fn task_series_create(&self, series: &TaskSeries) -> DbResult<()>;
    async fn task_series_delete(&self, id: TaskSeriesId) -> DbResult<()>;
}

#[async_trait]
pub trait TaskRepo: Send + Sync {
    async fn task_get(&self, id: TaskId) -> DbResult<Option<Task>>;
    async fn task_list(&self) -> DbResult<Vec<Task>>;
    async fn task_list_for_planting(&self, planting_id: PlantingId) -> DbResult<Vec<Task>>;
    async fn task_list_for_location(&self, location_id: LocationId) -> DbResult<Vec<Task>>;
    /// Tasks whose `planned_on` falls within `[from, to]` inclusive.
    /// Used by the task calendar (PR E).
    async fn task_list_in_range(
        &self,
        from: chrono::NaiveDate,
        to: chrono::NaiveDate,
    ) -> DbResult<Vec<Task>>;
    async fn task_create(&self, task: &Task) -> DbResult<()>;
    async fn task_update(&self, task: &Task) -> DbResult<()>;
    async fn task_delete(&self, id: TaskId) -> DbResult<()>;
}

#[async_trait]
pub trait TreatmentRepo: Send + Sync {
    async fn treatment_get(&self, id: TreatmentId) -> DbResult<Option<Treatment>>;
    async fn treatment_list_for_planting(
        &self,
        planting_id: PlantingId,
    ) -> DbResult<Vec<Treatment>>;
    async fn treatment_create(&self, treatment: &Treatment) -> DbResult<()>;
    async fn treatment_delete(&self, id: TreatmentId) -> DbResult<()>;
}

/// The append-only field-event journal (story 1.1). There is deliberately no
/// update or delete: corrections are new events, and the client-generated id
/// makes `field_event_create` idempotent (a replayed insert is a no-op).
#[async_trait]
pub trait FieldEventRepo: Send + Sync {
    /// Insert an event. Idempotent: a duplicate `id` is a silent no-op.
    async fn field_event_create(&self, event: &FieldEvent) -> DbResult<()>;
    async fn field_event_get(&self, id: FieldEventId) -> DbResult<Option<FieldEvent>>;
    /// Every event about a given target, oldest first (by `recorded_at`).
    async fn field_event_list_for_target(
        &self,
        target_kind: &str,
        target_id: Uuid,
    ) -> DbResult<Vec<FieldEvent>>;
    /// The whole journal, oldest first — used by backend migration (`copy_all`).
    async fn field_event_list_all(&self) -> DbResult<Vec<FieldEvent>>;
}

/// Aggregated trait that backends implement. Application code depends on
/// `dyn Repository` so backends can be swapped at runtime.
pub trait Repository:
    FamilyRepo
    + StrataRepo
    + LocationKindRepo
    + LocationRepo
    + CropRepo
    + VarietyRepo
    + PlantingRepo
    + YearlyHarvestRepo
    + TaskTypeRepo
    + TaskMethodRepo
    + TaskImplementRepo
    + TaskSeriesRepo
    + TaskRepo
    + TreatmentRepo
    + FieldEventRepo
    + Send
    + Sync
{
}
