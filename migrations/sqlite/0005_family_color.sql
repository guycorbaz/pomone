-- Pomone — per-family colour (mirrors Qrop's `family.color`).
--
-- A user-configurable colour used to tint plantings and the crop map by
-- botanical family. Additive `ADD COLUMN` with a NOT NULL default, so existing
-- rows get the neutral default and no rebuild is needed (project convention:
-- SQLite migrations stay ALTER-only). The domain validates the value as
-- `#RGB` / `#RRGGBB`; the DB only stores the text.
ALTER TABLE family ADD COLUMN color TEXT NOT NULL DEFAULT '#6B5D4D';
