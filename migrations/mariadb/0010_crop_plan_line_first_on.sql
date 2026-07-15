-- Crop-plan line anchor date (Epic 2, story 2.3). Mirrors
-- `migrations/sqlite/0010_crop_plan_line_first_on.sql`.
--
-- Additive, nullable: a line being entered (or a draft) may have no date yet.
-- The grid's derived date columns compute the N succession dates from
-- `first_on` + `stagger_days × k`; generation (2.6) anchors on it.
ALTER TABLE crop_plan_line ADD COLUMN first_on DATE NULL;
