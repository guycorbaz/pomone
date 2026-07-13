-- Append-only field-event journal (story 1.1) + additive task skip columns.
-- MariaDB mirror of migrations/sqlite/0007_field_event.sql — same semantics,
-- same literals. `field_event` rows are never updated or deleted; corrections
-- are new events pointing at the corrected one via `corrects`. No CHECK
-- constraints (0.6 audit): kind/target_kind validated in the domain + codec.
CREATE TABLE field_event (
    id           BINARY(16)   NOT NULL PRIMARY KEY,
    kind         VARCHAR(64)  NOT NULL,
    target_kind  VARCHAR(32)  NOT NULL,
    target_id    BINARY(16)   NOT NULL,
    occurred_at  DATE         NOT NULL,
    recorded_at  DATETIME(6)  NOT NULL,
    payload      TEXT         NOT NULL,
    corrects     BINARY(16),
    INDEX idx_field_event_target   (target_kind, target_id),
    INDEX idx_field_event_recorded (recorded_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- Additive skip-projection columns on `task` (written by facts::record_fact,
-- story 1.2). Nullable, no CHECK.
ALTER TABLE task ADD COLUMN skipped_on  DATE        NULL;
ALTER TABLE task ADD COLUMN skip_reason VARCHAR(32) NULL;
ALTER TABLE task ADD COLUMN skip_note   TEXT        NULL;
