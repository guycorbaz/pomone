-- Itinéraires techniques (ITK) — Epic 2, story 2.2. Mirrors
-- `migrations/sqlite/0009_itk.sql`.
--
-- `itk_template` = one ITK per crop (UNIQUE crop_id). `itk_activity` = an
-- ordered activity (task type at a signed day-offset from establishment),
-- optionally pinned to the revived dormant task_method/task_implement FKs
-- (no parallel columns). Invariants live in the domain, not CHECK constraints.
--
-- `offset_days` = domain i32 (INT matches). `position` = domain u32 stored as
-- INT (same convention as series/plants_count; 0-based ordering stays tiny). No
-- UNIQUE(template_id, position): ordering is `ORDER BY position, id`, and
-- uniqueness enforcement is deferred to the 2.5 editor's save path.
CREATE TABLE itk_template (
    id       BINARY(16) NOT NULL PRIMARY KEY,
    crop_id  BINARY(16) NOT NULL,
    UNIQUE KEY uq_itk_template_crop (crop_id),
    CONSTRAINT fk_itk_template_crop
        FOREIGN KEY (crop_id) REFERENCES crop(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE itk_activity (
    id            BINARY(16) NOT NULL PRIMARY KEY,
    template_id   BINARY(16) NOT NULL,
    task_type_id  BINARY(16) NOT NULL,
    offset_days   INT        NOT NULL,
    method_id     BINARY(16),
    implement_id  BINARY(16),
    label         TEXT,
    position      INT        NOT NULL,
    notes         TEXT,
    INDEX idx_itk_activity_template (template_id),
    CONSTRAINT fk_itk_activity_template
        FOREIGN KEY (template_id) REFERENCES itk_template(id) ON DELETE CASCADE,
    CONSTRAINT fk_itk_activity_task_type
        FOREIGN KEY (task_type_id) REFERENCES task_type(id) ON DELETE RESTRICT,
    CONSTRAINT fk_itk_activity_method
        FOREIGN KEY (method_id) REFERENCES task_method(id) ON DELETE SET NULL,
    CONSTRAINT fk_itk_activity_implement
        FOREIGN KEY (implement_id) REFERENCES task_implement(id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
