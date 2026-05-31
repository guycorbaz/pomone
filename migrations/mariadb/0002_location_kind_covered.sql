-- Pomone — add `covered` to location_kind.
--
-- Mirrors `migrations/sqlite/0002_location_kind_covered.sql`. Marks kinds that
-- grow under cover (greenhouse, tunnel…) so the home-page bed-usage curve can
-- split open-field beds from sheltered ones (issue #51). Existing rows default
-- to open-field (FALSE); the seed marks "Serre" as covered.

ALTER TABLE location_kind ADD COLUMN covered BOOLEAN NOT NULL DEFAULT FALSE;
