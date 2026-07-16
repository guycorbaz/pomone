-- Pomone — placement link on planned_planting (Epic 3, story 3.2).
--
-- When a planned succession is placed on a bed it becomes a real `planting`;
-- this nullable link records which one, so "unplaced" is a query
-- (`placed_planting_id IS NULL`) and placement is reversible & traceable
-- rather than a destructive delete. `ON DELETE SET NULL`: un-placing deletes
-- the planting and the row automatically returns to the unplaced list.
-- Additive, no CHECK, no trigger; default NULL. (SQLite permits a REFERENCES
-- clause on ADD COLUMN as long as the column is nullable with a NULL default.)
ALTER TABLE planned_planting
    ADD COLUMN placed_planting_id BLOB REFERENCES planting(id) ON DELETE SET NULL;
