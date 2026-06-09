-- Pomone — add a life-cycle `status` to planting.
--
-- A planting is no longer deleted once it carries real activity; instead it is
-- marked terminal (issue #63). Status is stored as TEXT mirroring the domain
-- `PlantingStatus` enum (snake_case). Existing rows default to 'active'.

ALTER TABLE planting ADD COLUMN status TEXT NOT NULL DEFAULT 'active'
    CHECK (status IN ('active', 'completed', 'failed', 'abandoned'));
