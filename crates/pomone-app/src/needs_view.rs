//! Presentation-layer helper for the **needs list** (Epic 2, story 2.7).
//!
//! The needs list answers the grower's ordering question: *for the whole
//! season, how much of each variety do I need, and by when must I have it?* It
//! aggregates the [`CropPlanLine`]s — **non-draft only**, placed or not — by
//! variety:
//!
//! - **quantity** = Σ over the variety's lines of `series × bed_meters`, the
//!   bed-geometry model of quantity (story 2.3). Kept as an exact [`Decimal`]
//!   and only stringified at the edge — no float rounding.
//! - **buy-by** = the earliest first-succession date across the variety's dated
//!   lines. `first_on` is the *sowing* date (`days_to_transplant` counts from
//!   sowing), so the seed/plants must be in hand by then — the deadline is
//!   *backward-computed* from the plan's earliest sow. A per-order lead time is
//!   not modeled yet, so this is the honest floor: buy no later than the first
//!   sow. A non-draft line with no date contributes its quantity but no
//!   deadline.
//!
//! Printing is deferred to Epic 4 (the PrintDoc pipeline); the UI offers a
//! disabled, tooltip-explained «print» affordance rather than a broken one.

use crate::error::AppResult;
use crate::i18n::I18n;
use chrono::NaiveDate;
use pomone_db::Repository;
use pomone_domain::{Crop, CropId, Variety, VarietyId};
use rust_decimal::Decimal;
use std::collections::HashMap;

/// One aggregated needs row — every field a UI-ready string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NeedRow {
    /// Stringified `VarietyId` (stable key; the UI may use it for navigation).
    pub variety_id: String,
    /// `"Crop · Variety"`.
    pub variety_label: String,
    /// Aggregated quantity in bed-meters, exact (normalized `Decimal`).
    pub quantity_bed_meters: String,
    /// Earliest sow date across the variety's dated lines, ISO `YYYY-MM-DD`,
    /// or empty when no line for this variety carries a date.
    pub buy_by: String,
    /// How many non-draft lines feed this variety, as a string.
    pub line_count: String,
}

/// Aggregate the non-draft plan lines into one row per variety, sorted by label
/// for a stable display. One DB read per lookup table.
pub async fn list_needs(repo: &dyn Repository, _i18n: &I18n) -> AppResult<Vec<NeedRow>> {
    let lines = repo.crop_plan_line_list().await?;
    let varieties = repo.variety_list().await?;
    let crops = repo.crop_list().await?;
    let var_by_id: HashMap<_, _> = varieties.iter().map(|v| (v.id, v)).collect();
    let crop_by_id: HashMap<_, _> = crops.iter().map(|c| (c.id, c)).collect();

    // Fold the non-draft lines into a per-variety accumulator. `Decimal` sums
    // stay exact; `buy_by` keeps the running minimum date. The multiply/add
    // saturate rather than panic: `series` is an unbounded `u32` and SQLite
    // stores `bed_meters` as free TEXT, so a pathological line could otherwise
    // overflow `Decimal::MAX` in this read path — same defensive posture as
    // `CropPlanLine::succession_dates`' cap.
    let mut acc: HashMap<VarietyId, Accumulator> = HashMap::new();
    for line in lines.iter().filter(|l| !l.draft) {
        let entry = acc.entry(line.variety_id).or_default();
        entry.quantity = entry
            .quantity
            .saturating_add(Decimal::from(line.series).saturating_mul(line.bed_meters));
        entry.line_count += 1;
        if let Some(date) = line.first_on {
            entry.buy_by = Some(entry.buy_by.map_or(date, |cur| cur.min(date)));
        }
    }

    let mut rows: Vec<NeedRow> = acc
        .into_iter()
        .map(|(variety_id, a)| NeedRow {
            variety_id: variety_id.to_string(),
            variety_label: variety_label(variety_id, &var_by_id, &crop_by_id),
            quantity_bed_meters: a.quantity.normalize().to_string(),
            buy_by: a
                .buy_by
                .map(|d| d.format("%Y-%m-%d").to_string())
                .unwrap_or_default(),
            line_count: a.line_count.to_string(),
        })
        .collect();

    // Sorted by label (then id) so the list is deterministic across reads.
    rows.sort_by(|x, y| {
        x.variety_label
            .cmp(&y.variety_label)
            .then_with(|| x.variety_id.cmp(&y.variety_id))
    });
    Ok(rows)
}

#[derive(Default)]
struct Accumulator {
    quantity: Decimal,
    buy_by: Option<NaiveDate>,
    line_count: u32,
}

/// `"Crop · Variety"`, falling back to `"?"` for a dangling reference.
fn variety_label(
    variety_id: VarietyId,
    var_by_id: &HashMap<VarietyId, &Variety>,
    crop_by_id: &HashMap<CropId, &Crop>,
) -> String {
    let Some(v) = var_by_id.get(&variety_id) else {
        return "?".to_owned();
    };
    let crop_name = crop_by_id.get(&v.crop_id).map_or("?", |c| c.name.as_str());
    format!("{crop_name} · {}", v.name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pomone_db::{
        seed_defaults, CropPlanLineRepo, CropRepo, FamilyRepo, SqliteRepository, VarietyRepo,
    };
    use pomone_domain::{
        AnnualProfile, Crop, CropPlanLine, Family, Lifespan, PruningSeason, Variety, VarietyProfile,
    };
    use rust_decimal::Decimal;
    use std::str::FromStr;

    fn i18n() -> I18n {
        I18n::new(crate::i18n::Lang::Fr).unwrap()
    }

    async fn variety(repo: &SqliteRepository, crop_name: &str, name: &str) -> VarietyId {
        let f = Family::new(format!("Fam-{crop_name}"), None, None).unwrap();
        repo.family_create(&f).await.unwrap();
        let crop = Crop::new(f.id, crop_name, None, Lifespan::Annual, PruningSeason::None).unwrap();
        repo.crop_create(&crop).await.unwrap();
        let v = Variety::new(
            crop.id,
            Lifespan::Annual,
            name,
            None,
            VarietyProfile::Annual(AnnualProfile::new(Some(20), 45, 30).unwrap()),
        )
        .unwrap();
        let id = v.id;
        repo.variety_create(&v).await.unwrap();
        id
    }

    fn line(
        vid: VarietyId,
        series: u32,
        meters: &str,
        first_on: Option<&str>,
        draft: bool,
    ) -> CropPlanLine {
        CropPlanLine::new(
            vid,
            series,
            Decimal::from_str(meters).unwrap(),
            7,
            first_on.map(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").unwrap()),
            draft,
            None,
        )
        .unwrap()
    }

    #[tokio::test]
    async fn aggregates_quantity_and_earliest_buy_by_per_variety() {
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        let lettuce = variety(&repo, "Laitue", "Batavia").await;
        let carrot = variety(&repo, "Carotte", "Nantaise").await;

        // Two lettuce lines: 3×15 + 2×10 = 65 bed-meters; earliest sow 2026-03-10.
        repo.crop_plan_line_create(&line(lettuce, 3, "15", Some("2026-04-01"), false))
            .await
            .unwrap();
        repo.crop_plan_line_create(&line(lettuce, 2, "10", Some("2026-03-10"), false))
            .await
            .unwrap();
        // One carrot line: 4×20 = 80; sow 2026-05-01.
        repo.crop_plan_line_create(&line(carrot, 4, "20", Some("2026-05-01"), false))
            .await
            .unwrap();

        let rows = list_needs(&repo, &i18n()).await.unwrap();
        assert_eq!(rows.len(), 2);
        // Sorted by label: "Carotte · Nantaise" before "Laitue · Batavia".
        assert_eq!(rows[0].variety_label, "Carotte · Nantaise");
        assert_eq!(rows[0].quantity_bed_meters, "80");
        assert_eq!(rows[0].buy_by, "2026-05-01");
        assert_eq!(rows[0].line_count, "1");

        assert_eq!(rows[1].variety_label, "Laitue · Batavia");
        assert_eq!(rows[1].quantity_bed_meters, "65");
        assert_eq!(rows[1].buy_by, "2026-03-10"); // earliest of the two
        assert_eq!(rows[1].line_count, "2");
    }

    #[tokio::test]
    async fn draft_lines_are_excluded() {
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        let v = variety(&repo, "Laitue", "Batavia").await;
        repo.crop_plan_line_create(&line(v, 3, "15", Some("2026-04-01"), false))
            .await
            .unwrap();
        // A draft line must not contribute quantity nor pull the buy-by earlier.
        repo.crop_plan_line_create(&line(v, 99, "99", Some("2026-01-01"), true))
            .await
            .unwrap();

        let rows = list_needs(&repo, &i18n()).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].quantity_bed_meters, "45");
        assert_eq!(rows[0].buy_by, "2026-04-01");
        assert_eq!(rows[0].line_count, "1");
    }

    #[tokio::test]
    async fn non_draft_dateless_line_has_quantity_but_empty_buy_by() {
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        let v = variety(&repo, "Laitue", "Batavia").await;
        repo.crop_plan_line_create(&line(v, 2, "12.5", None, false))
            .await
            .unwrap();

        let rows = list_needs(&repo, &i18n()).await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].quantity_bed_meters, "25"); // 2 × 12.5, exact
        assert_eq!(rows[0].buy_by, "");
    }

    #[tokio::test]
    async fn empty_when_no_non_draft_lines() {
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        assert!(list_needs(&repo, &i18n()).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn pathological_magnitude_saturates_instead_of_panicking() {
        // `series` (u32) is unbounded and SQLite stores `bed_meters` as free
        // TEXT, so `series × bed_meters` could exceed `Decimal::MAX`. The read
        // path must saturate, not panic.
        let repo = SqliteRepository::in_memory().await.unwrap();
        seed_defaults(&repo).await.unwrap();
        let v = variety(&repo, "Laitue", "Batavia").await;
        let huge = Decimal::MAX; // ~7.9e28
        let l = CropPlanLine::new(v, u32::MAX, huge, 7, None, false, None).unwrap();
        repo.crop_plan_line_create(&l).await.unwrap();

        let rows = list_needs(&repo, &i18n()).await.unwrap(); // must not panic
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].quantity_bed_meters,
            Decimal::MAX.normalize().to_string()
        );
    }
}
