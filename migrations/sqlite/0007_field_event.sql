-- Append-only field-event journal (story 1.1) + additive task skip columns.
--
-- `field_event` rows are NEVER updated or deleted: a correction is a new event
-- whose `corrects` points at the event it amends ("nothing is lost", D1). The
-- client-generated `id` is the idempotency key — a replayed insert is a no-op.
--
-- No CHECK constraints (see docs/check-constraint-audit.md, story 0.6): the
-- `kind` (FactKind) and `target_kind` literal sets are validated in the domain
-- and codec, so they can grow in later epics without a table rebuild.
CREATE TABLE field_event (
    id           BLOB    NOT NULL PRIMARY KEY,  -- client UUIDv4 (idempotency key)
    kind         TEXT    NOT NULL,              -- FactKind, dot-namespaced ('task.done'…)
    target_kind  TEXT    NOT NULL,              -- 'task' | 'planting' | …
    target_id    BLOB    NOT NULL,              -- referenced entity — NOT an FK: the journal outlives a deleted target
    occurred_at  TEXT    NOT NULL,              -- agronomic date (backdatable), ISO-8601
    recorded_at  TEXT    NOT NULL,              -- caller-injected instant, ISO-8601 datetime — audit + ordering
    payload      TEXT    NOT NULL,              -- JSON blob ('{}' when empty)
    corrects     BLOB                           -- nullable: id of the event this one corrects (not an FK)
) STRICT;

CREATE INDEX idx_field_event_target   ON field_event(target_kind, target_id);
CREATE INDEX idx_field_event_recorded ON field_event(recorded_at);

-- Additive skip-projection columns on `task`. Written by facts::record_fact
-- (story 1.2); read here from 1.1 on. Nullable, no CHECK — the SkipReason
-- literal set is validated in the codec, not the schema.
ALTER TABLE task ADD COLUMN skipped_on  TEXT;
ALTER TABLE task ADD COLUMN skip_reason TEXT;
ALTER TABLE task ADD COLUMN skip_note   TEXT;
