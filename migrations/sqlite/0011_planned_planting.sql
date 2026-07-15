-- Planned (generated, not-yet-placed) plantings (Epic 2, story 2.6).
--
-- One row per succession materialized from a crop-plan line: stagger-dated,
-- line-linked, without a bed/strata (placement is Epic 3). `(crop_plan_line_id,
-- series_index)` is UNIQUE so regeneration after a line edit updates the row in
-- place (non-destructive). No CHECK constraints; `bed_meters > 0` is enforced in
-- the domain constructor. Decimal-as-TEXT, same codec as the other decimals.
-- (Geometry/occupation_kind shifts to migration 0012.)
CREATE TABLE planned_planting (
    id                 BLOB    NOT NULL PRIMARY KEY,
    crop_plan_line_id  BLOB    NOT NULL REFERENCES crop_plan_line(id) ON DELETE CASCADE,
    variety_id         BLOB    NOT NULL REFERENCES variety(id)        ON DELETE RESTRICT,
    series_index       INTEGER NOT NULL,             -- 0-based succession index
    planned_on         TEXT    NOT NULL,             -- ISO-8601 stagger date
    bed_meters         TEXT    NOT NULL,             -- decimal-as-TEXT (> 0)
    UNIQUE (crop_plan_line_id, series_index)
) STRICT;

CREATE INDEX idx_planned_planting_line ON planned_planting(crop_plan_line_id);
