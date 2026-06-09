-- Pomone — move the vegetation `strata` from crop to planting (issue #86).
--
-- Mirrors migrations/sqlite/0004_strata_on_planting.sql. The training form /
-- height of a fruit tree depends on rootstock/training (decided at planting),
-- so strata belongs on the planting, not the crop — the same cultivar can sit
-- in different strata without duplicating the crop.
--
-- Existing plantings inherit their crop's stratum (planting → variety → crop).
-- Nullable at the DB level (kept symmetric with SQLite); the domain makes it
-- mandatory and the backfill leaves no NULLs.

-- 1) Add + backfill.
ALTER TABLE planting ADD COLUMN strata_id BINARY(16) NULL;

UPDATE planting p
JOIN variety v ON v.id = p.variety_id
JOIN crop c ON c.id = v.crop_id
SET p.strata_id = c.strata_id;

ALTER TABLE planting
    ADD INDEX idx_planting_strata (strata_id),
    ADD CONSTRAINT fk_planting_strata
        FOREIGN KEY (strata_id) REFERENCES strata(id) ON DELETE RESTRICT;

-- 2) Drop strata from crop (FK + index + column).
ALTER TABLE crop
    DROP FOREIGN KEY fk_crop_strata,
    DROP INDEX idx_crop_strata,
    DROP COLUMN strata_id;
