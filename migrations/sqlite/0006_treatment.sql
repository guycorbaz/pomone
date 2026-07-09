-- Phytosanitary treatment traceability (issue #82): one row per product
-- application on a planting — date, active substance, commercial product,
-- dose + explicit unit.
CREATE TABLE treatment (
    id                BLOB    NOT NULL PRIMARY KEY,
    planting_id       BLOB    NOT NULL REFERENCES planting(id) ON DELETE CASCADE,
    applied_on        TEXT    NOT NULL,
    active_substance  TEXT    NOT NULL,
    product_name      TEXT    NOT NULL,
    -- Decimal-as-TEXT (same codec as area_m2, labor_hours…).
    dose              TEXT    NOT NULL,
    dose_unit         TEXT    NOT NULL,
    notes             TEXT
) STRICT;

CREATE INDEX idx_treatment_planting   ON treatment(planting_id);
CREATE INDEX idx_treatment_applied_on ON treatment(applied_on);
