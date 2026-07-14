//! The virtual `PrintDoc` contract and the rough weekly print (story 1.4).
//!
//! [`WeekSheet`] is a **frozen, versioned data contract** (v1): a locale-neutral
//! projection of one week's tasks — grouped by day, then by bed (a
//! *tour-de-plaine*). Story 1.4 renders it as plain text; epic 4 will render the
//! same contract to PDF. Because it is pure data, the paper-loop harness can
//! assert its shape and use it as the `facts → PrintDoc` oracle.
//!
//! The DTO carries enums (`EntryState`, `SkipReason`) and dates, never localized
//! strings — every renderer localizes the chrome (weekday names, the skipped
//! word, reasons) itself. The bed/crop/task labels are user data (already in the
//! grower's language).

use crate::error::AppResult;
use crate::i18n::I18n;
use chrono::{Datelike, Duration, NaiveDate};
use fluent::FluentArgs;
use pomone_db::Repository;
use pomone_domain::{SkipReason, TaskId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Version of the [`WeekSheet`] data contract. Bump on any breaking shape
/// change; renderers (text now, PDF in epic 4) branch on it.
pub const PRINTDOC_VERSION: u32 = 1;

/// The settled state of a task, as it appears on the sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryState {
    /// Not done yet — an empty box (☐).
    Pending,
    /// Done — a ticked box (☒).
    Done,
    /// Deliberately skipped — struck out (⊘), with its reason.
    Skipped,
}

/// One task line on the sheet — a bed + crop + operation with its state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub task_id: TaskId,
    pub state: EntryState,
    /// Bed / location name; `None` for a bed-less (general) task.
    pub bed: Option<String>,
    /// Crop · variety label; `None` when the task isn't tied to a planting.
    pub crop: Option<String>,
    /// The operation (task-type name).
    pub task: String,
    /// Only set when `state == Skipped`.
    pub skip_reason: Option<SkipReason>,
}

/// One day of the week, with its entries ordered by bed then operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaySheet {
    pub date: NaiveDate,
    pub entries: Vec<Entry>,
}

/// The whole week — the frozen data contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeekSheet {
    pub version: u32,
    /// Monday of the ISO week.
    pub week_start: NaiveDate,
    /// Sunday of the ISO week.
    pub week_end: NaiveDate,
    /// The seven days, Monday→Sunday; days with no task carry an empty `entries`.
    pub days: Vec<DaySheet>,
}

/// Monday of the ISO week containing `reference`.
#[must_use]
pub fn week_start_of(reference: NaiveDate) -> NaiveDate {
    reference - Duration::days(i64::from(reference.weekday().num_days_from_monday()))
}

/// Build the week's [`WeekSheet`] from the real database — the tasks planned
/// Monday→Sunday of the week containing `reference`, projected to their settled
/// state, grouped by day then bed.
pub async fn build_week_sheet(repo: &dyn Repository, reference: NaiveDate) -> AppResult<WeekSheet> {
    let week_start = week_start_of(reference);
    let week_end = week_start + Duration::days(6);

    let tasks = repo.task_list_in_range(week_start, week_end).await?;

    // Label lookups (same resolution as the calendar view).
    let types: HashMap<_, _> = repo
        .task_type_list()
        .await?
        .into_iter()
        .map(|t| (t.id, t.name))
        .collect();
    let locations: HashMap<_, _> = repo
        .location_list()
        .await?
        .into_iter()
        .map(|l| (l.id, l.name))
        .collect();
    let plantings: HashMap<_, _> = repo
        .planting_list()
        .await?
        .into_iter()
        .map(|p| (p.id, p))
        .collect();
    let varieties: HashMap<_, _> = repo
        .variety_list()
        .await?
        .into_iter()
        .map(|v| (v.id, v))
        .collect();
    let crops: HashMap<_, _> = repo
        .crop_list()
        .await?
        .into_iter()
        .map(|c| (c.id, c.name))
        .collect();

    let mut by_day: HashMap<NaiveDate, Vec<Entry>> = HashMap::new();
    for task in tasks {
        let state = if task.completed_on.is_some() {
            EntryState::Done
        } else if task.skipped_on.is_some() {
            EntryState::Skipped
        } else {
            EntryState::Pending
        };
        let bed = task.location_id.and_then(|id| locations.get(&id).cloned());
        let crop = task.planting_id.and_then(|pid| {
            let planting = plantings.get(&pid)?;
            let variety = varieties.get(&planting.variety_id)?;
            let crop_name = crops
                .get(&variety.crop_id)
                .map_or("?", std::string::String::as_str);
            Some(format!("{crop_name} · {}", variety.name))
        });
        let entry = Entry {
            task_id: task.id,
            state,
            bed,
            crop,
            task: types
                .get(&task.task_type_id)
                .cloned()
                .unwrap_or_else(|| "?".to_owned()),
            skip_reason: if state == EntryState::Skipped {
                task.skip_reason
            } else {
                None
            },
        };
        by_day.entry(task.planned_on).or_default().push(entry);
    }

    let days = (0..7)
        .map(|offset| {
            let date = week_start + Duration::days(offset);
            let mut entries = by_day.remove(&date).unwrap_or_default();
            // Tour-de-plaine: bed first, then the operation.
            entries.sort_by(|a, b| {
                a.bed
                    .cmp(&b.bed)
                    .then_with(|| a.task.cmp(&b.task))
                    .then_with(|| a.task_id.cmp(&b.task_id))
            });
            DaySheet { date, entries }
        })
        .collect();

    Ok(WeekSheet {
        version: PRINTDOC_VERSION,
        week_start,
        week_end,
        days,
    })
}

/// Render a [`WeekSheet`] as the rough plain-text weekly print. Localizes the
/// chrome through `i18n`; the bed/crop/task labels are already the user's data.
#[must_use]
pub fn render_text(sheet: &WeekSheet, i18n: &I18n) -> String {
    let mut out = String::new();
    let title = {
        let mut args = FluentArgs::new();
        args.set("date", fmt_date(sheet.week_start, i18n));
        i18n.t_args("print-week-title", &args)
    };
    out.push_str(&title);
    out.push('\n');
    out.push_str(&"=".repeat(title.chars().count()));
    out.push('\n');

    let mut any = false;
    for day in &sheet.days {
        if day.entries.is_empty() {
            continue;
        }
        any = true;
        out.push('\n');
        out.push_str(&fmt_date(day.date, i18n));
        out.push('\n');
        for entry in &day.entries {
            out.push_str(&render_entry(entry, i18n));
            out.push('\n');
        }
    }
    if !any {
        out.push('\n');
        out.push_str(&i18n.t("print-empty-week"));
        out.push('\n');
    }
    out
}

fn render_entry(entry: &Entry, i18n: &I18n) -> String {
    let glyph = match entry.state {
        EntryState::Pending => '☐',
        EntryState::Done => '☒',
        EntryState::Skipped => '⊘',
    };
    let bed = entry.bed.clone().unwrap_or_else(|| i18n.t("print-no-bed"));
    let mut line = match &entry.crop {
        Some(crop) => format!("  {glyph} {bed} · {crop} — {}", entry.task),
        None => format!("  {glyph} {bed} — {}", entry.task),
    };
    if entry.state == EntryState::Skipped {
        use std::fmt::Write as _;
        let reason = entry
            .skip_reason
            .map(|r| i18n.t(&format!("skip-reason-{}", r.as_str())));
        let skipped = i18n.t("print-skipped");
        match reason {
            Some(reason) => write!(line, " ({skipped} : {reason})"),
            None => write!(line, " ({skipped})"),
        }
        .expect("writing to a String never fails");
    }
    line
}

/// The file name for the week's sheet, e.g. `pomone-semaine-2026-03-02.txt`.
#[must_use]
pub fn week_sheet_filename(week_start: NaiveDate) -> String {
    format!("pomone-semaine-{week_start}.txt")
}

/// The export ritual: build the week sheet from the real database, render it to
/// plain text, and write it into `dir` (created if missing). Returns the path.
pub async fn export_week_sheet(
    repo: &dyn Repository,
    i18n: &I18n,
    reference: NaiveDate,
    dir: &Path,
) -> AppResult<PathBuf> {
    let sheet = build_week_sheet(repo, reference).await?;
    let text = render_text(&sheet, i18n);
    std::fs::create_dir_all(dir)?;
    let path = dir.join(week_sheet_filename(sheet.week_start));
    std::fs::write(&path, text)?;
    Ok(path)
}

/// "Lundi 2 Mars 2026" — weekday + day + localized month + year.
fn fmt_date(date: NaiveDate, i18n: &I18n) -> String {
    let weekday = match date.weekday() {
        chrono::Weekday::Mon => "weekday-monday",
        chrono::Weekday::Tue => "weekday-tuesday",
        chrono::Weekday::Wed => "weekday-wednesday",
        chrono::Weekday::Thu => "weekday-thursday",
        chrono::Weekday::Fri => "weekday-friday",
        chrono::Weekday::Sat => "weekday-saturday",
        chrono::Weekday::Sun => "weekday-sunday",
    };
    let month = i18n.t(&format!("month-{}", date.month()));
    format!(
        "{} {} {} {}",
        i18n.t(weekday),
        date.day(),
        month,
        date.year()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use pomone_db::{seed_defaults, SqliteRepository, TaskRepo, TaskTypeRepo};
    use pomone_domain::Task;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn week_start_is_the_monday() {
        // 2026-03-04 is a Wednesday → Monday 2026-03-02.
        assert_eq!(week_start_of(d(2026, 3, 4)), d(2026, 3, 2));
        // A Monday maps to itself.
        assert_eq!(week_start_of(d(2026, 3, 2)), d(2026, 3, 2));
        // A Sunday maps back to its Monday.
        assert_eq!(week_start_of(d(2026, 3, 8)), d(2026, 3, 2));
    }

    async fn repo_with_task(planned_on: NaiveDate) -> (SqliteRepository, TaskId) {
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        let task_type = repo.task_type_list().await.unwrap()[0].id;
        let task = Task::new(
            None, None, task_type, None, None, planned_on, None, None, None, None,
        );
        repo.task_create(&task).await.unwrap();
        (repo, task.id)
    }

    #[tokio::test]
    async fn build_projects_states_and_groups_by_day() {
        let (repo, task_id) = repo_with_task(d(2026, 3, 4)).await;
        // A pending task on Wednesday of the 2026-03-02 week.
        let sheet = build_week_sheet(&repo, d(2026, 3, 2)).await.unwrap();
        assert_eq!(sheet.version, PRINTDOC_VERSION);
        assert_eq!(sheet.week_start, d(2026, 3, 2));
        assert_eq!(sheet.week_end, d(2026, 3, 8));
        assert_eq!(sheet.days.len(), 7);
        let wed = &sheet.days[2];
        assert_eq!(wed.date, d(2026, 3, 4));
        assert_eq!(wed.entries.len(), 1);
        assert_eq!(wed.entries[0].task_id, task_id);
        assert_eq!(wed.entries[0].state, EntryState::Pending);
    }

    #[tokio::test]
    async fn a_task_outside_the_week_is_excluded() {
        let (repo, _) = repo_with_task(d(2026, 3, 9)).await; // next Monday
        let sheet = build_week_sheet(&repo, d(2026, 3, 2)).await.unwrap();
        assert!(sheet.days.iter().all(|day| day.entries.is_empty()));
    }

    #[test]
    fn render_shows_the_three_states() {
        let i18n = I18n::new(crate::i18n::Lang::Fr).unwrap();
        let sheet = WeekSheet {
            version: PRINTDOC_VERSION,
            week_start: NaiveDate::from_ymd_opt(2026, 3, 2).unwrap(),
            week_end: NaiveDate::from_ymd_opt(2026, 3, 8).unwrap(),
            days: vec![DaySheet {
                date: NaiveDate::from_ymd_opt(2026, 3, 2).unwrap(),
                entries: vec![
                    Entry {
                        task_id: TaskId::new(),
                        state: EntryState::Pending,
                        bed: Some("Planche A".into()),
                        crop: Some("Tomate · Marmande".into()),
                        task: "Semis".into(),
                        skip_reason: None,
                    },
                    Entry {
                        task_id: TaskId::new(),
                        state: EntryState::Skipped,
                        bed: Some("Planche C".into()),
                        crop: None,
                        task: "Désherbage".into(),
                        skip_reason: Some(SkipReason::Weather),
                    },
                ],
            }],
        };
        let text = render_text(&sheet, &i18n);
        assert!(text.contains('☐'), "pending box");
        assert!(text.contains('⊘'), "skipped glyph");
        assert!(text.contains("Planche A · Tomate · Marmande — Semis"));
        // Skip reason localized + rendered (fr: "météo").
        assert!(text.contains("météo"), "localized skip reason:\n{text}");
        // Header carries the localized week title.
        assert!(text.starts_with("Semaine du"), "title:\n{text}");
    }

    #[tokio::test]
    async fn export_writes_a_dated_file() {
        let (repo, _) = repo_with_task(d(2026, 3, 4)).await;
        let i18n = I18n::new(crate::i18n::Lang::Fr).unwrap();
        let mut dir = std::env::temp_dir();
        dir.push(format!("pomone_printdoc_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = export_week_sheet(&repo, &i18n, d(2026, 3, 2), &dir)
            .await
            .unwrap();
        assert_eq!(
            path.file_name().unwrap().to_str().unwrap(),
            "pomone-semaine-2026-03-02.txt"
        );
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.starts_with("Semaine du"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
