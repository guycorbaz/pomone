-- Pomone — add `covered` to location_kind.
--
-- Marks kinds that grow under cover (greenhouse, tunnel…) so the home-page
-- bed-usage curve can split open-field beds from sheltered ones (issue #51).
-- Stored as INTEGER 0/1 (SQLite has no native boolean); existing rows default
-- to open-field (0). The seed marks "Serre" as covered.

ALTER TABLE location_kind ADD COLUMN covered INTEGER NOT NULL DEFAULT 0;
