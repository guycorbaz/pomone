-- Pomone — initial MariaDB schema.
--
-- UUIDs are stored as BINARY(16). Dates use the native DATE type. Decimals
-- use DECIMAL(20,6) which fits any agricultural quantity (areas, weights,
-- yields, prices). Engine is InnoDB for FK enforcement; charset utf8mb4 for
-- accents and botanical Latin.
--
-- Mirrors `migrations/sqlite/0001_initial.sql` semantically; dialect-only
-- differences are confined to this file.

CREATE TABLE family (
    id            BINARY(16)   NOT NULL PRIMARY KEY,
    name          VARCHAR(255) NOT NULL,
    latin_name    VARCHAR(255),
    description   TEXT
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE strata (
    id            BINARY(16)   NOT NULL PRIMARY KEY,
    name          VARCHAR(255) NOT NULL,
    description   TEXT,
    min_height_m  DECIMAL(20,6),
    max_height_m  DECIMAL(20,6),
    sort_order    INT          NOT NULL DEFAULT 0
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE location_kind (
    id            BINARY(16)   NOT NULL PRIMARY KEY,
    name          VARCHAR(255) NOT NULL,
    description   TEXT
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE location (
    id            BINARY(16)   NOT NULL PRIMARY KEY,
    parent_id     BINARY(16),
    kind_id       BINARY(16)   NOT NULL,
    name          VARCHAR(255) NOT NULL,
    length_m      DECIMAL(20,6) NOT NULL,
    width_m       DECIMAL(20,6) NOT NULL,
    notes         TEXT,
    INDEX idx_location_parent (parent_id),
    INDEX idx_location_kind   (kind_id),
    CONSTRAINT fk_location_parent
        FOREIGN KEY (parent_id) REFERENCES location(id) ON DELETE RESTRICT,
    CONSTRAINT fk_location_kind
        FOREIGN KEY (kind_id) REFERENCES location_kind(id) ON DELETE RESTRICT
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE crop (
    id                    BINARY(16)   NOT NULL PRIMARY KEY,
    family_id             BINARY(16)   NOT NULL,
    strata_id             BINARY(16)   NOT NULL,
    name                  VARCHAR(255) NOT NULL,
    latin_name            VARCHAR(255),
    pruning_season        VARCHAR(16)  NOT NULL,
    lifespan_kind         VARCHAR(16)  NOT NULL,
    lifespan_years        INT,
    productive_pattern    VARCHAR(32),
    years_to_first_yield  INT,
    INDEX idx_crop_family (family_id),
    INDEX idx_crop_strata (strata_id),
    CONSTRAINT fk_crop_family
        FOREIGN KEY (family_id) REFERENCES family(id) ON DELETE RESTRICT,
    CONSTRAINT fk_crop_strata
        FOREIGN KEY (strata_id) REFERENCES strata(id) ON DELETE RESTRICT,
    CONSTRAINT chk_crop_pruning
        CHECK (pruning_season IN ('winter','summer','both','none')),
    CONSTRAINT chk_crop_lifespan_shape
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
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE variety (
    id                            BINARY(16)   NOT NULL PRIMARY KEY,
    crop_id                       BINARY(16)   NOT NULL,
    name                          VARCHAR(255) NOT NULL,
    description                   TEXT,
    profile_kind                  VARCHAR(16)  NOT NULL,
    days_to_transplant            INT,
    days_to_maturity              INT,
    harvest_window_days           INT,
    bud_break_doy                 INT,
    flowering_doy                 INT,
    harvest_start_doy             INT,
    harvest_end_doy               INT,
    expected_yield_kg_per_plant   DECIMAL(20,6),
    INDEX idx_variety_crop (crop_id),
    CONSTRAINT fk_variety_crop
        FOREIGN KEY (crop_id) REFERENCES crop(id) ON DELETE CASCADE,
    CONSTRAINT chk_variety_profile_shape
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
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE planting (
    id                       BINARY(16)    NOT NULL PRIMARY KEY,
    variety_id               BINARY(16)    NOT NULL,
    location_id              BINARY(16)    NOT NULL,
    area_m2                  DECIMAL(20,6) NOT NULL,
    plants_count             INT           NOT NULL,
    name                     VARCHAR(255),
    notes                    TEXT,
    schedule_kind            VARCHAR(16)   NOT NULL,
    sown_on                  DATE,
    transplanted_on          DATE,
    first_harvest_on         DATE,
    last_harvest_on          DATE,
    established_on           DATE,
    expected_removal_on      DATE,
    INDEX idx_planting_variety  (variety_id),
    INDEX idx_planting_location (location_id),
    CONSTRAINT fk_planting_variety
        FOREIGN KEY (variety_id) REFERENCES variety(id) ON DELETE RESTRICT,
    CONSTRAINT fk_planting_location
        FOREIGN KEY (location_id) REFERENCES location(id) ON DELETE RESTRICT,
    CONSTRAINT chk_planting_schedule_shape
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
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE yearly_harvest (
    planting_id        BINARY(16)    NOT NULL,
    year               INT           NOT NULL,
    expected_yield_kg  DECIMAL(20,6),
    actual_yield_kg    DECIMAL(20,6),
    notes              TEXT,
    PRIMARY KEY (planting_id, year),
    CONSTRAINT fk_yearly_harvest_planting
        FOREIGN KEY (planting_id) REFERENCES planting(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ---------------------------------------------------------------
-- Tasks (operations) — three taxonomy tables (task_type, task_method,
-- task_implement) feed the main `task` table. See pomone-domain for
-- the matching enum / structs and the SQLite mirror for the design
-- notes (FK rules, category enum, etc.).
-- ---------------------------------------------------------------

CREATE TABLE task_type (
    id        BINARY(16)  NOT NULL PRIMARY KEY,
    name      VARCHAR(255) NOT NULL UNIQUE,
    category  VARCHAR(16)  NOT NULL,
    color     VARCHAR(7)   NOT NULL,
    INDEX idx_task_type_category (category),
    CONSTRAINT chk_task_type_category CHECK (category IN
        ('sow','transplant','harvest','weeding','irrigation',
         'treatment','tillage','other')),
    CONSTRAINT chk_task_type_color_length
        CHECK (CHAR_LENGTH(color) IN (4, 7))
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE task_method (
    id     BINARY(16)   NOT NULL PRIMARY KEY,
    name   VARCHAR(255) NOT NULL UNIQUE,
    notes  TEXT
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE task_implement (
    id     BINARY(16)   NOT NULL PRIMARY KEY,
    name   VARCHAR(255) NOT NULL UNIQUE,
    notes  TEXT
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- Recurring task series — see sqlite/0001 for the design rationale.
-- `task.series_id` (added below) links each occurrence back to the
-- template; deleting a series sets that column to NULL rather than
-- cascading so historical occurrences stay queryable.
CREATE TABLE task_series (
    id                  BINARY(16)    NOT NULL PRIMARY KEY,
    planting_id         BINARY(16),
    location_id         BINARY(16),
    task_type_id        BINARY(16)    NOT NULL,
    task_method_id      BINARY(16),
    implement_id        BINARY(16),
    recurrence_unit     VARCHAR(16)   NOT NULL,
    recurrence_interval INT           NOT NULL,
    first_planned_on    DATE          NOT NULL,
    end_on              DATE,
    notes               TEXT,
    INDEX idx_task_series_planting (planting_id),
    INDEX idx_task_series_location (location_id),
    INDEX idx_task_series_type     (task_type_id),
    CONSTRAINT fk_task_series_planting
        FOREIGN KEY (planting_id) REFERENCES planting(id) ON DELETE CASCADE,
    CONSTRAINT fk_task_series_location
        FOREIGN KEY (location_id) REFERENCES location(id) ON DELETE CASCADE,
    CONSTRAINT fk_task_series_type
        FOREIGN KEY (task_type_id) REFERENCES task_type(id) ON DELETE RESTRICT,
    CONSTRAINT fk_task_series_method
        FOREIGN KEY (task_method_id) REFERENCES task_method(id) ON DELETE SET NULL,
    CONSTRAINT fk_task_series_implement
        FOREIGN KEY (implement_id) REFERENCES task_implement(id) ON DELETE SET NULL,
    CONSTRAINT chk_task_series_unit
        CHECK (recurrence_unit IN ('days','weeks','months')),
    CONSTRAINT chk_task_series_interval
        CHECK (recurrence_interval >= 1)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE task (
    id              BINARY(16)    NOT NULL PRIMARY KEY,
    planting_id     BINARY(16),
    location_id     BINARY(16),
    task_type_id    BINARY(16)    NOT NULL,
    task_method_id  BINARY(16),
    implement_id    BINARY(16),
    series_id       BINARY(16),
    planned_on      DATE          NOT NULL,
    completed_on    DATE,
    duration_min    INT,
    labor_hours     DECIMAL(20,6),
    notes           TEXT,
    INDEX idx_task_planting     (planting_id),
    INDEX idx_task_location     (location_id),
    INDEX idx_task_type         (task_type_id),
    INDEX idx_task_series       (series_id),
    INDEX idx_task_planned_on   (planned_on),
    INDEX idx_task_completed_on (completed_on),
    CONSTRAINT fk_task_planting
        FOREIGN KEY (planting_id) REFERENCES planting(id) ON DELETE CASCADE,
    CONSTRAINT fk_task_location
        FOREIGN KEY (location_id) REFERENCES location(id) ON DELETE CASCADE,
    CONSTRAINT fk_task_type
        FOREIGN KEY (task_type_id) REFERENCES task_type(id) ON DELETE RESTRICT,
    CONSTRAINT fk_task_method
        FOREIGN KEY (task_method_id) REFERENCES task_method(id) ON DELETE SET NULL,
    CONSTRAINT fk_task_implement
        FOREIGN KEY (implement_id) REFERENCES task_implement(id) ON DELETE SET NULL,
    CONSTRAINT fk_task_series
        FOREIGN KEY (series_id) REFERENCES task_series(id) ON DELETE SET NULL
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
