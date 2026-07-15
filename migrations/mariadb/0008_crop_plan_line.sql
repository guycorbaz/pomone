-- Crop-plan lines (Epic 2, story 2.1). Mirrors
-- `migrations/sqlite/0008_crop_plan_line.sql`.
--
-- A line is a planned `variety × series × bed_meters`, `stagger_days` apart —
-- NOT a planting yet (generation comes in story 2.6). `draft` is orthogonal to
-- validity. Invariants (`series ≥ 1`, `bed_meters > 0`) live in the domain
-- constructor, not in CHECK constraints. ITK template tables land with story
-- 2.2 (migration 0009).
CREATE TABLE crop_plan_line (
    id           BINARY(16)    NOT NULL PRIMARY KEY,
    variety_id   BINARY(16)    NOT NULL,
    series       INT           NOT NULL,
    bed_meters   DECIMAL(20,6) NOT NULL,
    stagger_days INT           NOT NULL,
    draft        BOOLEAN       NOT NULL,
    notes        TEXT,
    INDEX idx_crop_plan_line_variety (variety_id),
    CONSTRAINT fk_crop_plan_line_variety
        FOREIGN KEY (variety_id) REFERENCES variety(id) ON DELETE RESTRICT
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
