# Changelog

All notable changes to Pomone are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/), and the project aims to follow
[Semantic Versioning](https://semver.org/).

## [0.9.0] — Unreleased

First pre-production release: feature-complete for the v1.0.0 scope, now in a
hardening phase ahead of real-world testing.

### Added

- **Unified calendar** — a single monthly “Calendrier” showing operational
  tasks *and* crop-cycle milestones, de-duplicated at the source. Drag a task
  to reschedule it, filter by task category, toggle the milestone family, and
  hover an entry for its full label; the month bar shows a legend and counts.
- **Home bed-usage curve** — replaces the old “Aperçu” counters with a weekly
  occupancy curve over the season (open-field vs sheltered beds), drawn on the
  same axis as the season Gantt below it.
- **Flat “Tâches” list** — every task, newest first, with “overdue” / “today”
  badges; the calendar covers the grid view.
- **Backup & restore** — `pomone-cli backup` / `restore` snapshot and restore
  the SQLite database (a reversible file copy).
- **Confirmation dialog** — destructive deletes (strata, task, task type) now
  ask for confirmation.
- Location kinds carry a **`covered`** flag (greenhouses/tunnels), feeding the
  sheltered-beds curve.

### Changed

- Service error messages are **localized** (fr/en) instead of leaking raw
  English `Display` strings.
- The backend migration **refuses a non-empty target** up front rather than
  corrupting it mid-copy.
- Deleting a task type reports “in use” only on an actual foreign-key
  violation; other failures surface as real errors.
- Opening the user manual and failed crop-map moves now report their outcome
  in the status banner.
- The Gantt / today-line / bed-usage curve share a consistent 365-day axis.

### Fixed

- Recurring-task series no longer stop materializing future occurrences on
  Feb 29 (the rolling horizon collapsed to “today”).

### Removed

- The standalone “Calendrier” (harvest-event) screen, merged into the unified
  calendar.
- ~23 orphaned localization keys left by the home/calendar/agenda rewrites.

[0.9.0]: https://github.com/guycorbaz/pomone/releases/tag/v0.9.0
