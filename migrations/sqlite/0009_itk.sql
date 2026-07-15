-- Itinéraires techniques (ITK) — Epic 2, story 2.2.
--
-- `itk_template` is a crop's ITK container (one per crop, enforced by the UNIQUE
-- on crop_id). `itk_activity` is an ordered activity: a task type at a signed
-- day-offset from establishment, optionally pinned to a method/implement — the
-- **dormant** task_method/task_implement FKs, revived here (no parallel
-- columns). No CHECK constraints; invariants live in the domain (`itk.rs`).
--
-- Split from the planned "0008_planning" (see 0008_crop_plan_line.sql): each
-- table lands with its own domain, and migrations are immutable once applied.
--
-- `offset_days` is the domain's `i32` (INT matches exactly, negatives included).
-- `position` is a `u32` stored as INTEGER (same convention as series/plants_count;
-- MariaDB mirrors with `INT` — a value above 2^31 would diverge, but 0-based
-- ordering never reaches it). No `UNIQUE(template_id, position)`: ordering is
-- deterministic via `ORDER BY position, id`, and uniqueness enforcement belongs
-- to the story-2.5 editor's save path (see itk.rs).
CREATE TABLE itk_template (
    id       BLOB NOT NULL PRIMARY KEY,
    crop_id  BLOB NOT NULL UNIQUE REFERENCES crop(id) ON DELETE CASCADE
) STRICT;

CREATE TABLE itk_activity (
    id            BLOB    NOT NULL PRIMARY KEY,
    template_id   BLOB    NOT NULL REFERENCES itk_template(id)   ON DELETE CASCADE,
    task_type_id  BLOB    NOT NULL REFERENCES task_type(id)      ON DELETE RESTRICT,
    -- Signed offset from establishment: J-10 = -10, J+20 = 20, 0 = on the day.
    offset_days   INTEGER NOT NULL,
    -- Dormant FKs revived (optional; SET NULL mirrors task.task_method_id).
    method_id     BLOB    REFERENCES task_method(id)    ON DELETE SET NULL,
    implement_id  BLOB    REFERENCES task_implement(id) ON DELETE SET NULL,
    label         TEXT,
    position      INTEGER NOT NULL,   -- explicit ordering within the template
    notes         TEXT
) STRICT;

CREATE INDEX idx_itk_activity_template ON itk_activity(template_id);
