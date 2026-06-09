//! Presentation-layer helpers for the Calendar screen.
//!
//! Produces a flat list of `CalendarEvent`s for a date range, derived from
//! the `Planting` schedules in the repository:
//!
//! * `Cycle` schedules contribute sowing / transplanting / harvest start /
//!   harvest end events on the dates stored on the planting.
//! * `Perennial` schedules contribute establishment and (optional) removal
//!   events, plus per-year bud-break / flowering / harvest start / harvest
//!   end events for productive years that overlap the requested range
//!   (using the variety's `PluriannualProfile` DOYs).
//!
//! Events carry a stringified `PlantingId` so the UI can navigate to a
//! planting later without needing to know about `Uuid`.

use crate::error::AppResult;
use chrono::{Datelike, NaiveDate};
use pomone_db::Repository;
use pomone_domain::date_calc::{date_from_doy, productive_years};
use pomone_domain::{Lifespan, PlantingSchedule, ProductivePattern, VarietyProfile};
use std::collections::HashMap;

/// Kind of event surfaced on a calendar cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalendarEventKind {
    Sowing,
    Transplanting,
    HarvestStart,
    HarvestEnd,
    Establishment,
    Removal,
    BudBreak,
    Flowering,
}

/// One event ready to render on a calendar day cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarEvent {
    pub date: NaiveDate,
    pub kind: CalendarEventKind,
    /// Human-friendly label, typically `"<Crop> · <Variety>"`.
    pub label: String,
    /// Stringified `PlantingId` so the UI can route to a planting later.
    pub planting_id: String,
}

/// Return all events that fall in `[start, end]` (inclusive), sorted by date.
///
/// One DB read each for plantings, varieties and crops — same shape as
/// `list_plantings`, fine for Pomone's scale.
// Eight event kinds × two schedule variants × in-range filtering naturally
// produces a long body; splitting it further would harm readability.
#[allow(clippy::too_many_lines)]
pub async fn list_events_in_range(
    repo: &dyn Repository,
    start: NaiveDate,
    end: NaiveDate,
) -> AppResult<Vec<CalendarEvent>> {
    let plantings = repo.planting_list().await?;
    let varieties = repo.variety_list().await?;
    let crops = repo.crop_list().await?;

    let var_by_id: HashMap<_, _> = varieties.iter().map(|v| (v.id, v)).collect();
    let crop_by_id: HashMap<_, _> = crops.iter().map(|c| (c.id, c)).collect();

    let mut events: Vec<CalendarEvent> = Vec::new();

    for p in &plantings {
        let Some(variety) = var_by_id.get(&p.variety_id) else {
            continue;
        };
        let crop = crop_by_id.get(&variety.crop_id);
        let crop_name = crop.map_or("?", |c| c.name.as_str());
        let label = format!("{crop_name} · {}", variety.name);
        let planting_id = p.id.to_string();

        match p.schedule {
            PlantingSchedule::Cycle {
                sown_on,
                transplanted_on,
                first_harvest_on,
                last_harvest_on,
            } => {
                push_if_in_range(
                    &mut events,
                    sown_on,
                    CalendarEventKind::Sowing,
                    &label,
                    &planting_id,
                    start,
                    end,
                );
                push_if_in_range(
                    &mut events,
                    transplanted_on,
                    CalendarEventKind::Transplanting,
                    &label,
                    &planting_id,
                    start,
                    end,
                );
                push_if_in_range(
                    &mut events,
                    Some(first_harvest_on),
                    CalendarEventKind::HarvestStart,
                    &label,
                    &planting_id,
                    start,
                    end,
                );
                push_if_in_range(
                    &mut events,
                    Some(last_harvest_on),
                    CalendarEventKind::HarvestEnd,
                    &label,
                    &planting_id,
                    start,
                    end,
                );
            }
            PlantingSchedule::Perennial {
                established_on,
                expected_removal_on,
            } => {
                push_if_in_range(
                    &mut events,
                    Some(established_on),
                    CalendarEventKind::Establishment,
                    &label,
                    &planting_id,
                    start,
                    end,
                );
                push_if_in_range(
                    &mut events,
                    expected_removal_on,
                    CalendarEventKind::Removal,
                    &label,
                    &planting_id,
                    start,
                    end,
                );

                if let VarietyProfile::Pluriannual(profile) = &variety.profile {
                    // Non-recurring pluriannuals never reach a yielding year
                    // here — productive_years would return empty. Skip cleanly.
                    let Some(Lifespan::Pluriannual {
                        pattern:
                            ProductivePattern::Recurring {
                                years_to_first_yield,
                            },
                        lifespan_years,
                    }) = crop.map(|c| c.lifespan)
                    else {
                        continue;
                    };
                    let years = productive_years(
                        established_on,
                        years_to_first_yield,
                        expected_removal_on,
                        lifespan_years,
                    );
                    for y in years {
                        // The harvest window can wrap into year `y + 1`, so
                        // accept any year where any event could land in
                        // [start, end].
                        if y < start.year() - 1 || y > end.year() {
                            continue;
                        }
                        if let Some(doy) = profile.bud_break_doy {
                            if let Ok(d) = date_from_doy(y, doy) {
                                push_if_in_range(
                                    &mut events,
                                    Some(d),
                                    CalendarEventKind::BudBreak,
                                    &label,
                                    &planting_id,
                                    start,
                                    end,
                                );
                            }
                        }
                        if let Some(doy) = profile.flowering_doy {
                            if let Ok(d) = date_from_doy(y, doy) {
                                push_if_in_range(
                                    &mut events,
                                    Some(d),
                                    CalendarEventKind::Flowering,
                                    &label,
                                    &planting_id,
                                    start,
                                    end,
                                );
                            }
                        }
                        if let Ok(d) = date_from_doy(y, profile.harvest_start_doy) {
                            push_if_in_range(
                                &mut events,
                                Some(d),
                                CalendarEventKind::HarvestStart,
                                &label,
                                &planting_id,
                                start,
                                end,
                            );
                        }
                        // Wrap end DOY into the next calendar year if it
                        // precedes the start DOY (e.g. olive Oct..Feb).
                        let end_year = if profile.harvest_end_doy >= profile.harvest_start_doy {
                            y
                        } else {
                            y + 1
                        };
                        if let Ok(d) = date_from_doy(end_year, profile.harvest_end_doy) {
                            push_if_in_range(
                                &mut events,
                                Some(d),
                                CalendarEventKind::HarvestEnd,
                                &label,
                                &planting_id,
                                start,
                                end,
                            );
                        }
                    }
                }
            }
        }
    }

    events.sort_by(|a, b| a.date.cmp(&b.date).then_with(|| a.label.cmp(&b.label)));
    Ok(events)
}

fn push_if_in_range(
    out: &mut Vec<CalendarEvent>,
    date: Option<NaiveDate>,
    kind: CalendarEventKind,
    label: &str,
    planting_id: &str,
    start: NaiveDate,
    end: NaiveDate,
) {
    if let Some(d) = date {
        if d >= start && d <= end {
            out.push(CalendarEvent {
                date: d,
                kind,
                label: label.to_owned(),
                planting_id: planting_id.to_owned(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::create_annual_planting_from_sowing;
    use crate::test_helpers::seed_test_data;
    use pomone_db::{seed_defaults, LocationRepo, SqliteRepository, StrataRepo, VarietyRepo};
    use rust_decimal_macros::dec;

    async fn fresh_repo() -> SqliteRepository {
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        repo
    }

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[tokio::test]
    async fn empty_range_returns_no_events() {
        let repo = fresh_repo().await;
        seed_test_data(&repo).await.unwrap();
        let events = list_events_in_range(&repo, d(2026, 1, 1), d(2026, 12, 31))
            .await
            .unwrap();
        assert!(events.is_empty());
    }

    #[tokio::test]
    async fn annual_planting_emits_sowing_and_harvest_events() {
        let repo = fresh_repo().await;
        seed_test_data(&repo).await.unwrap();

        let varieties = repo.variety_list().await.unwrap();
        let locations = repo.location_list().await.unwrap();
        let bed = locations.iter().find(|l| l.parent_id.is_some()).unwrap();
        let strata = repo.strata_list().await.unwrap()[0].id;
        create_annual_planting_from_sowing(
            &repo,
            varieties[0].id,
            bed.id,
            strata,
            d(2026, 3, 1),
            dec!(20),
            100,
            Some("démo".to_owned()),
            None,
        )
        .await
        .unwrap();

        let events = list_events_in_range(&repo, d(2026, 1, 1), d(2026, 12, 31))
            .await
            .unwrap();

        let kinds: Vec<_> = events.iter().map(|e| e.kind).collect();
        assert!(kinds.contains(&CalendarEventKind::Sowing));
        assert!(kinds.contains(&CalendarEventKind::HarvestStart));
        assert!(kinds.contains(&CalendarEventKind::HarvestEnd));
        assert!(events.iter().all(|e| e.label.starts_with("Tomate · ")));
    }

    #[tokio::test]
    async fn range_filters_out_events_outside_window() {
        let repo = fresh_repo().await;
        seed_test_data(&repo).await.unwrap();

        let varieties = repo.variety_list().await.unwrap();
        let locations = repo.location_list().await.unwrap();
        let bed = locations.iter().find(|l| l.parent_id.is_some()).unwrap();
        let strata = repo.strata_list().await.unwrap()[0].id;
        create_annual_planting_from_sowing(
            &repo,
            varieties[0].id,
            bed.id,
            strata,
            d(2026, 3, 1),
            dec!(20),
            100,
            None,
            None,
        )
        .await
        .unwrap();

        // March 2026 only — the sowing event lands here, harvest does not.
        let events = list_events_in_range(&repo, d(2026, 3, 1), d(2026, 3, 31))
            .await
            .unwrap();
        let kinds: Vec<_> = events.iter().map(|e| e.kind).collect();
        assert!(kinds.contains(&CalendarEventKind::Sowing));
        assert!(!kinds.contains(&CalendarEventKind::HarvestStart));
    }

    #[tokio::test]
    async fn events_are_sorted_by_date() {
        let repo = fresh_repo().await;
        seed_test_data(&repo).await.unwrap();

        let varieties = repo.variety_list().await.unwrap();
        let locations = repo.location_list().await.unwrap();
        let bed = locations.iter().find(|l| l.parent_id.is_some()).unwrap();
        let strata = repo.strata_list().await.unwrap()[0].id;
        create_annual_planting_from_sowing(
            &repo,
            varieties[0].id,
            bed.id,
            strata,
            d(2026, 4, 1),
            dec!(20),
            100,
            None,
            None,
        )
        .await
        .unwrap();
        create_annual_planting_from_sowing(
            &repo,
            varieties[1].id,
            bed.id,
            strata,
            d(2026, 3, 1),
            dec!(20),
            100,
            None,
            None,
        )
        .await
        .unwrap();

        let events = list_events_in_range(&repo, d(2026, 1, 1), d(2026, 12, 31))
            .await
            .unwrap();
        let dates: Vec<_> = events.iter().map(|e| e.date).collect();
        let mut sorted = dates.clone();
        sorted.sort();
        assert_eq!(dates, sorted);
    }
}
