-- Planned (generated, not-yet-placed) plantings (Epic 2, story 2.6). Mirrors
-- `migrations/sqlite/0011_planned_planting.sql`.
--
-- One row per generated succession of a crop-plan line: stagger-dated,
-- line-linked, unplaced. `(crop_plan_line_id, series_index)` UNIQUE makes
-- regeneration update-in-place (non-destructive). `bed_meters > 0` lives in the
-- domain, not a CHECK. Geometry shifts to migration 0012.
CREATE TABLE planned_planting (
    id                 BINARY(16)    NOT NULL PRIMARY KEY,
    crop_plan_line_id  BINARY(16)    NOT NULL,
    variety_id         BINARY(16)    NOT NULL,
    series_index       INT           NOT NULL,
    planned_on         DATE          NOT NULL,
    bed_meters         DECIMAL(20,6) NOT NULL,
    UNIQUE KEY uq_planned_planting_succession (crop_plan_line_id, series_index),
    INDEX idx_planned_planting_line (crop_plan_line_id),
    CONSTRAINT fk_planned_planting_line
        FOREIGN KEY (crop_plan_line_id) REFERENCES crop_plan_line(id) ON DELETE CASCADE,
    CONSTRAINT fk_planned_planting_variety
        FOREIGN KEY (variety_id) REFERENCES variety(id) ON DELETE RESTRICT
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
