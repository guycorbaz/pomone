-- Crop-plan lines (Epic 2, story 2.1): the winter plan, one row per intention.
--
-- A line is a planned `variety × series × bed_meters`, `stagger_days` apart —
-- NOT a planting yet (generation into staggered plantings comes in story 2.6).
-- `draft` is orthogonal to validity (a valid line can still be a draft the
-- grower is refining; drafts are excluded from generation and the needs list).
--
-- No CHECK constraints (docs/check-constraint-audit.md): `series ≥ 1` and
-- `bed_meters > 0` are enforced in the domain constructor (`CropPlanLine::new`).
-- Quantity is bed-meters in R1; a polymorphic occupancy discriminant is
-- deferred. The ITK template tables land with story 2.2 (migration 0009).
--
-- `series`/`stagger_days` are the domain's `u32` but stored as INTEGER, same as
-- `plants_count`/`duration_min` (realistic counts are tiny; MariaDB's mirror
-- uses `INT` — a value above 2^31 would diverge, but that's unreachable input).
-- `bed_meters` is decimal-as-TEXT: exact here, `DECIMAL(20,6)` on MariaDB — the
-- established codec tradeoff (dose, area_m2…), fine for bed-meters.
CREATE TABLE crop_plan_line (
    id           BLOB    NOT NULL PRIMARY KEY,
    variety_id   BLOB    NOT NULL REFERENCES variety(id) ON DELETE RESTRICT,
    series       INTEGER NOT NULL,             -- number of successions (≥ 1)
    -- Decimal-as-TEXT (same codec as area_m2, dose, labor_hours…).
    bed_meters   TEXT    NOT NULL,             -- bed-meters per succession (> 0)
    stagger_days INTEGER NOT NULL,             -- days between successions (≥ 0)
    draft        INTEGER NOT NULL,             -- 0/1, orthogonal to validity
    notes        TEXT
) STRICT;

CREATE INDEX idx_crop_plan_line_variety ON crop_plan_line(variety_id);
