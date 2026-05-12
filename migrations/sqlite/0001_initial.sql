-- Pomone — initial SQLite schema.
--
-- All UUIDs are stored as 16-byte BLOBs. Dates are stored as ISO-8601 TEXT
-- strings (chrono's `NaiveDate` default). Decimals are stored as TEXT to
-- preserve precision (sqlx behaviour for SQLite).
--
-- Foreign-key enforcement is enabled per-connection by `pomone-db` (SQLite
-- default is off). All tables use STRICT for type safety.

CREATE TABLE family (
    id            BLOB    NOT NULL PRIMARY KEY,
    name          TEXT    NOT NULL,
    latin_name    TEXT,
    description   TEXT
) STRICT;

CREATE TABLE strata (
    id            BLOB    NOT NULL PRIMARY KEY,
    name          TEXT    NOT NULL,
    description   TEXT,
    min_height_m  TEXT,           -- Decimal as text
    max_height_m  TEXT,
    sort_order    INTEGER NOT NULL DEFAULT 0
) STRICT;

CREATE TABLE location_kind (
    id            BLOB    NOT NULL PRIMARY KEY,
    name          TEXT    NOT NULL,
    description   TEXT
) STRICT;

CREATE TABLE location (
    id            BLOB    NOT NULL PRIMARY KEY,
    parent_id     BLOB    REFERENCES location(id) ON DELETE RESTRICT,
    kind_id       BLOB    NOT NULL REFERENCES location_kind(id) ON DELETE RESTRICT,
    name          TEXT    NOT NULL,
    length_m      TEXT    NOT NULL,
    width_m       TEXT    NOT NULL,
    notes         TEXT
) STRICT;

CREATE INDEX idx_location_parent ON location(parent_id);
CREATE INDEX idx_location_kind   ON location(kind_id);

CREATE TABLE crop (
    id                    BLOB    NOT NULL PRIMARY KEY,
    family_id             BLOB    NOT NULL REFERENCES family(id) ON DELETE RESTRICT,
    strata_id             BLOB    NOT NULL REFERENCES strata(id) ON DELETE RESTRICT,
    name                  TEXT    NOT NULL,
    latin_name            TEXT,
    pruning_season        TEXT    NOT NULL CHECK (pruning_season IN ('winter','summer','both','none')),
    -- Lifespan encoding:
    lifespan_kind         TEXT    NOT NULL CHECK (lifespan_kind IN ('annual','pluriannual')),
    lifespan_years        INTEGER,                 -- NULL when annual
    productive_pattern    TEXT,                    -- NULL when annual; 'single_cycle' | 'recurring'
    years_to_first_yield  INTEGER,                 -- only for recurring
    CHECK (
        (lifespan_kind = 'annual'
         AND lifespan_years IS NULL
         AND productive_pattern IS NULL
         AND years_to_first_yield IS NULL)
        OR
        (lifespan_kind = 'pluriannual'
         AND lifespan_years IS NOT NULL
         AND productive_pattern IN ('single_cycle','recurring')
         AND ((productive_pattern = 'single_cycle' AND years_to_first_yield IS NULL)
              OR (productive_pattern = 'recurring'    AND years_to_first_yield IS NOT NULL)))
    )
) STRICT;

CREATE INDEX idx_crop_family ON crop(family_id);
CREATE INDEX idx_crop_strata ON crop(strata_id);

CREATE TABLE variety (
    id                            BLOB    NOT NULL PRIMARY KEY,
    crop_id                       BLOB    NOT NULL REFERENCES crop(id) ON DELETE CASCADE,
    name                          TEXT    NOT NULL,
    description                   TEXT,
    profile_kind                  TEXT    NOT NULL CHECK (profile_kind IN ('annual','pluriannual')),
    -- Annual profile:
    days_to_transplant            INTEGER,
    days_to_maturity              INTEGER,
    harvest_window_days           INTEGER,
    -- Pluriannual profile:
    bud_break_doy                 INTEGER,
    flowering_doy                 INTEGER,
    harvest_start_doy             INTEGER,
    harvest_end_doy               INTEGER,
    expected_yield_kg_per_plant   TEXT,
    CHECK (
        (profile_kind = 'annual'
         AND days_to_maturity IS NOT NULL
         AND harvest_window_days IS NOT NULL
         AND bud_break_doy IS NULL
         AND flowering_doy IS NULL
         AND harvest_start_doy IS NULL
         AND harvest_end_doy IS NULL
         AND expected_yield_kg_per_plant IS NULL)
        OR
        (profile_kind = 'pluriannual'
         AND harvest_start_doy IS NOT NULL
         AND harvest_end_doy IS NOT NULL
         AND days_to_transplant IS NULL
         AND days_to_maturity IS NULL
         AND harvest_window_days IS NULL)
    )
) STRICT;

CREATE INDEX idx_variety_crop ON variety(crop_id);

CREATE TABLE planting (
    id                       BLOB    NOT NULL PRIMARY KEY,
    variety_id               BLOB    NOT NULL REFERENCES variety(id) ON DELETE RESTRICT,
    location_id              BLOB    NOT NULL REFERENCES location(id) ON DELETE RESTRICT,
    area_m2                  TEXT    NOT NULL,
    plants_count             INTEGER NOT NULL,
    name                     TEXT,
    notes                    TEXT,
    schedule_kind            TEXT    NOT NULL CHECK (schedule_kind IN ('cycle','perennial')),
    -- Cycle dates:
    sown_on                  TEXT,
    transplanted_on          TEXT,
    first_harvest_on         TEXT,
    last_harvest_on          TEXT,
    -- Perennial dates:
    established_on           TEXT,
    expected_removal_on      TEXT,
    CHECK (
        (schedule_kind = 'cycle'
         AND first_harvest_on IS NOT NULL
         AND last_harvest_on  IS NOT NULL
         AND established_on   IS NULL
         AND expected_removal_on IS NULL)
        OR
        (schedule_kind = 'perennial'
         AND established_on IS NOT NULL
         AND first_harvest_on IS NULL
         AND last_harvest_on  IS NULL
         AND sown_on          IS NULL
         AND transplanted_on  IS NULL)
    )
) STRICT;

CREATE INDEX idx_planting_variety  ON planting(variety_id);
CREATE INDEX idx_planting_location ON planting(location_id);

CREATE TABLE yearly_harvest (
    planting_id        BLOB    NOT NULL REFERENCES planting(id) ON DELETE CASCADE,
    year               INTEGER NOT NULL,
    expected_yield_kg  TEXT,
    actual_yield_kg    TEXT,
    notes              TEXT,
    PRIMARY KEY (planting_id, year)
) STRICT;
