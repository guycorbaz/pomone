-- Phytosanitary treatment traceability (issue #82): one row per product
-- application on a planting — date, active substance, commercial product,
-- dose + explicit unit.
CREATE TABLE treatment (
    id                BINARY(16)    NOT NULL PRIMARY KEY,
    planting_id       BINARY(16)    NOT NULL,
    applied_on        DATE          NOT NULL,
    active_substance  VARCHAR(255)  NOT NULL,
    product_name      VARCHAR(255)  NOT NULL,
    dose              DECIMAL(20,6) NOT NULL,
    dose_unit         VARCHAR(64)   NOT NULL,
    notes             TEXT,
    INDEX idx_treatment_planting   (planting_id),
    INDEX idx_treatment_applied_on (applied_on),
    CONSTRAINT fk_treatment_planting
        FOREIGN KEY (planting_id) REFERENCES planting(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
