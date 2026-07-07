-- Pomone — per-family colour (mirrors Qrop's `family.color`).
--
-- Mirrors migrations/sqlite/0005_family_color.sql. A user-configurable colour
-- used to tint plantings and the crop map by botanical family. Additive column
-- with a NOT NULL default so existing rows inherit the neutral default. The
-- domain validates the value as `#RGB` / `#RRGGBB`; the DB only stores the text.
ALTER TABLE family ADD COLUMN color VARCHAR(7) NOT NULL DEFAULT '#6B5D4D';
