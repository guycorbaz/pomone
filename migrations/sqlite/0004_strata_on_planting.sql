-- Pomone — move the vegetation `strata` from crop to planting (issue #86).
--
-- The training form / height of a fruit tree (basse-tige, mi-tige…) depends on
-- rootstock and training, decided when planting — so the same cultivar can sit
-- in different strata. Carrying strata on the planting (not the crop) avoids
-- duplicating a whole crop per height.
--
-- Existing plantings inherit their crop's stratum (planting → variety → crop).
-- Done with ALTER only (no table rebuild): the migrations run with
-- `PRAGMA foreign_keys=ON` inside a transaction, where the SQLite 12-step
-- rebuild (which needs foreign_keys=OFF) is awkward. The column is nullable at
-- the DB level — the domain (`Planting`) makes `strata_id` mandatory, and the
-- backfill leaves no NULLs since every variety→crop→strata link is NOT NULL.

-- 1) Add the column with its FK (allowed under foreign_keys=ON because the
--    default is NULL), then backfill from each planting's crop.
ALTER TABLE planting ADD COLUMN strata_id BLOB REFERENCES strata(id) ON DELETE RESTRICT;

UPDATE planting
SET strata_id = (
    SELECT c.strata_id
    FROM variety v
    JOIN crop c ON c.id = v.crop_id
    WHERE v.id = planting.variety_id
);

CREATE INDEX idx_planting_strata ON planting(strata_id);

-- 2) Drop strata from crop (index first, then the column / its FK).
DROP INDEX idx_crop_strata;
ALTER TABLE crop DROP COLUMN strata_id;
