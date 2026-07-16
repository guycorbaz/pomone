-- Pomone — location geometry: add `occupation_kind` (Epic 3, story 3.1).
--
-- Mirrors `migrations/sqlite/0012_geometry.sql`. The discriminant the capacity
-- engine reads to know how a location's capacity is measured. R1 has a single
-- value, `bed-meters` (footprint on a leaf bed = the bed's `length_m`); the
-- column exists so future polymorphism (tree rows, hectares) is an additive
-- enum variant, not another migration. Additive, no CHECK, no trigger.
-- Existing rows default to `bed-meters`.
ALTER TABLE location ADD COLUMN occupation_kind VARCHAR(32) NOT NULL DEFAULT 'bed-meters';
