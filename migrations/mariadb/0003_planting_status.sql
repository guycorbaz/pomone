-- Pomone — add a life-cycle `status` to planting.
--
-- Mirrors `migrations/sqlite/0003_planting_status.sql`. A planting is no longer
-- deleted once it carries real activity; instead it is marked terminal (issue
-- #63). Status mirrors the domain `PlantingStatus` enum (snake_case). Existing
-- rows default to 'active'.

ALTER TABLE planting ADD COLUMN status VARCHAR(16) NOT NULL DEFAULT 'active'
    CHECK (status IN ('active', 'completed', 'failed', 'abandoned'));
