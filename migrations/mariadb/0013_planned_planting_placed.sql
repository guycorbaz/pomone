-- Pomone — placement link on planned_planting (Epic 3, story 3.2).
--
-- Mirrors `migrations/sqlite/0013_planned_planting_placed.sql`. Nullable link
-- from a planned succession to the real `planting` it was placed as, so
-- "unplaced" is a query (`placed_planting_id IS NULL`) and placement is
-- reversible: `ON DELETE SET NULL` returns the row to the unplaced list when
-- the planting is deleted. Additive, no CHECK, no trigger; default NULL.
ALTER TABLE planned_planting
    ADD COLUMN placed_planting_id BINARY(16) NULL,
    ADD CONSTRAINT fk_planned_planting_placed
        FOREIGN KEY (placed_planting_id) REFERENCES planting(id) ON DELETE SET NULL;
