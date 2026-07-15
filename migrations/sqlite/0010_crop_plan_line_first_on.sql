-- Crop-plan line anchor date (Epic 2, story 2.3): the first succession's date.
--
-- Additive, nullable: a line being entered (or a draft) may not have a date yet.
-- The plan grid's derived date columns compute the N succession dates from
-- `first_on` + `stagger_days × k`; generation (story 2.6) anchors on it. ISO-8601
-- date-as-TEXT, same codec as other dates. (Geometry/occupation_kind shifts to
-- migration 0011.)
ALTER TABLE crop_plan_line ADD COLUMN first_on TEXT;
