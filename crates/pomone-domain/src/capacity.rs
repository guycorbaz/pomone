//! The pure capacity engine — soil occupancy over time, covered/open split,
//! recursive hierarchy aggregation, and peak composition.
//!
//! This module is **pure functions only**: no `Repository`, no I/O, no clock.
//! Inputs are plain values ([`Placement`]s), outputs are plain values. It is
//! the UI-independent core the placement screen (story 3.2) and the
//! occupancy-map document (story 7.2) render on top of. It never duplicates
//! date arithmetic — it consumes [`crate::date_calc`] for the one place it
//! turns a [`crate::PlantingSchedule`] into an interval ([`occupancy_window`]);
//! the engine core itself only *compares* dates.
//!
//! # The model
//!
//! A [`Placement`] occupies a **leaf bed** over a **half-open interval**
//! `[start, end)` — `end` is *exclusive*, which is what makes two adjacent
//! successions (`[a, b)` then `[b, c)`) not double-count the shared instant
//! `b`. Its `path` lists the leaf and every ancestor up to the root, so
//! occupancy aggregates up the hierarchy for free: querying a leaf sums that
//! bed, querying the root sums everything beneath it (FR12). A perennial
//! without a removal date carries `end = None` and is resolved against a
//! caller-supplied `horizon`.
//!
//! # Defensive posture (Epic-2 read-path lesson)
//!
//! `bed_meters` comes from persisted rows (SQLite stores decimals as free
//! TEXT) and intervals can span decades. Every sum here **saturates** — a
//! pathological-but-persisted footprint or a ±50-year interval must never
//! panic or overflow. See [`CoverSplit`].

use crate::date_calc::add_days;
use crate::error::DomainResult;
use crate::ids::LocationId;
use crate::planting::PlantingSchedule;
use chrono::NaiveDate;
use rust_decimal::Decimal;

/// Occupied bed-metres at an instant, split by cover. Both fields are
/// non-negative running totals; every accumulation **saturates** so an absurd
/// persisted `bed_meters` or an enormous placement count can never overflow.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CoverSplit {
    /// Bed-metres occupied under cover (greenhouse, tunnel…).
    pub covered: Decimal,
    /// Bed-metres occupied in open field.
    pub open: Decimal,
}

impl CoverSplit {
    /// Total occupied bed-metres, covered + open (saturating).
    #[must_use]
    pub fn total(&self) -> Decimal {
        self.covered.saturating_add(self.open)
    }

    /// Add one placement's footprint to the right side of the split (saturating).
    fn add(&mut self, bed_meters: Decimal, covered: bool) {
        if covered {
            self.covered = self.covered.saturating_add(bed_meters);
        } else {
            self.open = self.open.saturating_add(bed_meters);
        }
    }
}

/// One placed planting for the engine: a leaf bed, a footprint, a cover flag,
/// and a half-open occupancy interval. Built by the caller (story 3.2) from a
/// real planting + its bed geometry; the engine treats every field as opaque
/// data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placement {
    /// The leaf bed and its ancestors, **leaf first, root last**. Occupancy at
    /// any `location` sums the placements whose `path` contains it, which is
    /// exactly the hierarchy aggregation (a parent sums all descendant beds).
    pub path: Vec<LocationId>,
    /// Running bed-metres this planting occupies on its leaf bed (the R1
    /// geometry rule: `length_m` of the bed). Assumed `>= 0`; saturating maths
    /// make even an absurd value safe.
    pub bed_meters: Decimal,
    /// `true` iff the bed is sheltered (its kind or any ancestor's kind is
    /// covered — see [`crate::is_sheltered`]).
    pub covered: bool,
    /// Occupancy start (inclusive).
    pub start: NaiveDate,
    /// Occupancy end (**exclusive**). `None` = open-ended (perennial with no
    /// removal date), resolved against the engine's `horizon`.
    pub end: Option<NaiveDate>,
}

impl Placement {
    /// The resolved exclusive end: the placement's own `end`, or `horizon` when
    /// open-ended.
    #[must_use]
    fn resolved_end(&self, horizon: NaiveDate) -> NaiveDate {
        self.end.unwrap_or(horizon)
    }

    /// Is this placement occupying ground at instant `t`? Half-open:
    /// `start <= t < end`. An empty/inverted interval (`start >= end`) is never
    /// active.
    #[must_use]
    fn active_at(&self, t: NaiveDate, horizon: NaiveDate) -> bool {
        let end = self.resolved_end(horizon);
        self.start <= t && t < end
    }

    /// Does this placement sit at or beneath `location`?
    #[must_use]
    fn under(&self, location: LocationId) -> bool {
        self.path.contains(&location)
    }

    /// Public predicate: is this placement occupying ground at instant `t`?
    /// Half-open `[start, end)`, with `None` end resolved against `horizon`.
    /// Lets read-path callers (e.g. `capacity_view`) compose peaks without
    /// re-deriving the interval logic the engine owns.
    #[must_use]
    pub fn covers(&self, t: NaiveDate, horizon: NaiveDate) -> bool {
        self.active_at(t, horizon)
    }
}

/// Occupied bed-metres at `location` and instant `t`, split covered/open.
///
/// Sums every placement that (a) sits at or beneath `location` in the hierarchy
/// and (b) is active at `t`. Querying a leaf gives that bed's load; querying an
/// interior node gives the aggregate of all beds beneath it (FR12). Order of
/// `placements` is irrelevant (the sum is commutative).
#[must_use]
pub fn occupancy_at(
    placements: &[Placement],
    location: LocationId,
    t: NaiveDate,
    horizon: NaiveDate,
) -> CoverSplit {
    let mut split = CoverSplit::default();
    for p in placements {
        if p.under(location) && p.active_at(t, horizon) {
            split.add(p.bed_meters, p.covered);
        }
    }
    split
}

/// The placements composing the load at `location` and instant `t` — the data
/// the placement screen and PeakPanel (FR11) render as "which series make up
/// this peak". Returned in input order (stable).
#[must_use]
pub fn composition_at(
    placements: &[Placement],
    location: LocationId,
    t: NaiveDate,
    horizon: NaiveDate,
) -> Vec<&Placement> {
    placements
        .iter()
        .filter(|p| p.under(location) && p.active_at(t, horizon))
        .collect()
}

/// A point on the occupancy timeline: the instant and the load there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PeakPoint {
    /// The date at which this load holds.
    pub at: NaiveDate,
    /// The covered/open load at that date.
    pub load: CoverSplit,
}

/// The peak (maximum total) occupancy at `location` over the half-open window
/// `[window_start, window_end)`.
///
/// Occupancy is a step function that only jumps **up** at a placement's start
/// and **down** at its (resolved) end, so its maximum over the window is
/// attained either at the window start (for placements already running) or at
/// some placement start inside the window. We evaluate only those candidate
/// instants. Ties resolve to the **earliest** date. Returns `None` iff the
/// window is empty (`window_start >= window_end`).
#[must_use]
pub fn peak(
    placements: &[Placement],
    location: LocationId,
    window_start: NaiveDate,
    window_end: NaiveDate,
    horizon: NaiveDate,
) -> Option<PeakPoint> {
    if window_start >= window_end {
        return None;
    }

    // Candidate instants: the window start, plus every distinct placement start
    // that falls strictly inside the window. Sorted ascending so that, keeping
    // the first strict maximum, ties resolve to the earliest date.
    let mut candidates: Vec<NaiveDate> = vec![window_start];
    for p in placements {
        if p.start > window_start && p.start < window_end {
            candidates.push(p.start);
        }
    }
    candidates.sort_unstable();
    candidates.dedup();

    let mut best: Option<PeakPoint> = None;
    for at in candidates {
        let load = occupancy_at(placements, location, at, horizon);
        let improves = best.map_or(true, |b| load.total() > b.load.total());
        if improves {
            best = Some(PeakPoint { at, load });
        }
    }
    best
}

/// Map a [`PlantingSchedule`] to its bed-occupancy interval `(start, end)`,
/// where `end` is **exclusive** (`None` = open-ended perennial, resolved
/// against the engine `horizon`).
///
/// This is the one place the engine's inputs touch `PlantingSchedule`; it is
/// deliberately a *separate* helper so the engine core stays independent of the
/// schedule's shape.
///
/// - **Cycle:** the bed is occupied from establishment on the bed —
///   transplanting if any, else sowing, else first harvest — through the last
///   harvest day inclusive, i.e. `end = last_harvest_on + 1`.
/// - **Perennial:** from `established_on` to `expected_removal_on` exclusive
///   (the ground is free again on the removal day); no removal date → `None`.
///
/// Returns [`crate::DomainError::DateOverflow`] only if `last_harvest_on` is the
/// maximum representable date.
pub fn occupancy_window(
    schedule: &PlantingSchedule,
) -> DomainResult<(NaiveDate, Option<NaiveDate>)> {
    match schedule {
        PlantingSchedule::Cycle {
            sown_on,
            transplanted_on,
            first_harvest_on,
            last_harvest_on,
        } => {
            let start = transplanted_on.or(*sown_on).unwrap_or(*first_harvest_on);
            let end = add_days(*last_harvest_on, 1)?;
            Ok((start, Some(end)))
        }
        PlantingSchedule::Perennial {
            established_on,
            expected_removal_on,
        } => Ok((*established_on, *expected_removal_on)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use rust_decimal_macros::dec;
    use uuid::Uuid;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    /// Build a placement on a single leaf under a single root.
    fn placed(
        leaf: LocationId,
        root: LocationId,
        bed_meters: Decimal,
        covered: bool,
        start: NaiveDate,
        end: Option<NaiveDate>,
    ) -> Placement {
        Placement {
            path: vec![leaf, root],
            bed_meters,
            covered,
            start,
            end,
        }
    }

    const HORIZON: fn() -> NaiveDate = || NaiveDate::from_ymd_opt(2100, 1, 1).unwrap();

    // ---- occupancy_at -------------------------------------------------------

    #[test]
    fn occupancy_counts_active_placement() {
        let leaf = LocationId::new();
        let root = LocationId::new();
        let ps = vec![placed(
            leaf,
            root,
            dec!(15),
            false,
            d(2026, 4, 1),
            Some(d(2026, 7, 1)),
        )];
        // Inside the interval.
        let mid = occupancy_at(&ps, leaf, d(2026, 5, 1), HORIZON());
        assert_eq!(mid.open, dec!(15));
        assert_eq!(mid.covered, dec!(0));
        assert_eq!(mid.total(), dec!(15));
    }

    #[test]
    fn half_open_end_is_exclusive() {
        let leaf = LocationId::new();
        let root = LocationId::new();
        let ps = vec![placed(
            leaf,
            root,
            dec!(15),
            false,
            d(2026, 4, 1),
            Some(d(2026, 7, 1)),
        )];
        // Start day counts…
        assert_eq!(
            occupancy_at(&ps, leaf, d(2026, 4, 1), HORIZON()).total(),
            dec!(15)
        );
        // …end day does NOT (exclusive).
        assert_eq!(
            occupancy_at(&ps, leaf, d(2026, 7, 1), HORIZON()).total(),
            dec!(0)
        );
        // …day before the start does not.
        assert_eq!(
            occupancy_at(&ps, leaf, d(2026, 3, 31), HORIZON()).total(),
            dec!(0)
        );
    }

    #[test]
    fn adjacent_successions_never_double_count() {
        // [a,b) then [b,c): at instant b only the second is active.
        let leaf = LocationId::new();
        let root = LocationId::new();
        let a = placed(
            leaf,
            root,
            dec!(10),
            false,
            d(2026, 4, 1),
            Some(d(2026, 6, 1)),
        );
        let b = placed(
            leaf,
            root,
            dec!(20),
            false,
            d(2026, 6, 1),
            Some(d(2026, 8, 1)),
        );
        let ps = vec![a, b];
        // Boundary instant b: first ended (exclusive), second started.
        assert_eq!(
            occupancy_at(&ps, leaf, d(2026, 6, 1), HORIZON()).total(),
            dec!(20)
        );
    }

    #[test]
    fn covered_and_open_split_apart() {
        let leaf_c = LocationId::new();
        let leaf_o = LocationId::new();
        let root = LocationId::new();
        let ps = vec![
            placed(
                leaf_c,
                root,
                dec!(12),
                true,
                d(2026, 1, 1),
                Some(d(2027, 1, 1)),
            ),
            placed(
                leaf_o,
                root,
                dec!(30),
                false,
                d(2026, 1, 1),
                Some(d(2027, 1, 1)),
            ),
        ];
        let at_root = occupancy_at(&ps, root, d(2026, 6, 1), HORIZON());
        assert_eq!(at_root.covered, dec!(12));
        assert_eq!(at_root.open, dec!(30));
        assert_eq!(at_root.total(), dec!(42));
    }

    // ---- hierarchy ----------------------------------------------------------

    #[test]
    fn parent_aggregates_children() {
        let root = LocationId::new();
        let bed1 = LocationId::new();
        let bed2 = LocationId::new();
        let ps = vec![
            placed(
                bed1,
                root,
                dec!(15),
                false,
                d(2026, 4, 1),
                Some(d(2026, 9, 1)),
            ),
            placed(
                bed2,
                root,
                dec!(25),
                false,
                d(2026, 4, 1),
                Some(d(2026, 9, 1)),
            ),
        ];
        let t = d(2026, 5, 1);
        assert_eq!(occupancy_at(&ps, bed1, t, HORIZON()).total(), dec!(15));
        assert_eq!(occupancy_at(&ps, bed2, t, HORIZON()).total(), dec!(25));
        // Root = sum of the two beds.
        assert_eq!(occupancy_at(&ps, root, t, HORIZON()).total(), dec!(40));
    }

    // ---- open-ended perennial + horizon ------------------------------------

    #[test]
    fn open_ended_perennial_occupies_to_horizon() {
        let leaf = LocationId::new();
        let root = LocationId::new();
        let ps = vec![placed(leaf, root, dec!(8), false, d(1996, 3, 1), None)];
        // Decades later, still active up to the horizon.
        assert_eq!(
            occupancy_at(&ps, leaf, d(2026, 6, 1), HORIZON()).total(),
            dec!(8)
        );
        // At the horizon itself: exclusive, so free.
        assert_eq!(
            occupancy_at(&ps, leaf, HORIZON(), HORIZON()).total(),
            dec!(0)
        );
    }

    #[test]
    fn horizon_extension_does_not_change_the_past() {
        let leaf = LocationId::new();
        let root = LocationId::new();
        let ps = vec![placed(leaf, root, dec!(8), false, d(1996, 3, 1), None)];
        let t = d(2026, 6, 1);
        let near = occupancy_at(&ps, leaf, t, d(2050, 1, 1));
        let far = occupancy_at(&ps, leaf, t, d(2200, 1, 1));
        assert_eq!(near, far);
    }

    // ---- composition + peak -------------------------------------------------

    #[test]
    fn composition_lists_the_active_series() {
        let leaf = LocationId::new();
        let root = LocationId::new();
        let a = placed(
            leaf,
            root,
            dec!(10),
            false,
            d(2026, 4, 1),
            Some(d(2026, 6, 1)),
        );
        let b = placed(
            leaf,
            root,
            dec!(20),
            false,
            d(2026, 5, 1),
            Some(d(2026, 8, 1)),
        );
        let ps = vec![a, b];
        // Overlap window May: both active.
        let comp = composition_at(&ps, root, d(2026, 5, 15), HORIZON());
        assert_eq!(comp.len(), 2);
        // April: only the first.
        let comp = composition_at(&ps, root, d(2026, 4, 15), HORIZON());
        assert_eq!(comp.len(), 1);
        assert_eq!(comp[0].bed_meters, dec!(10));
    }

    #[test]
    fn peak_finds_the_overlap_maximum_and_its_date() {
        let leaf = LocationId::new();
        let root = LocationId::new();
        let a = placed(
            leaf,
            root,
            dec!(10),
            false,
            d(2026, 4, 1),
            Some(d(2026, 6, 1)),
        );
        let b = placed(
            leaf,
            root,
            dec!(20),
            false,
            d(2026, 5, 1),
            Some(d(2026, 8, 1)),
        );
        let ps = vec![a, b];
        let pk = peak(&ps, root, d(2026, 1, 1), d(2026, 12, 31), HORIZON()).unwrap();
        // The peak is the May overlap: 10 + 20 = 30, first reached on 1 May.
        assert_eq!(pk.load.total(), dec!(30));
        assert_eq!(pk.at, d(2026, 5, 1));
    }

    #[test]
    fn peak_on_empty_window_is_none() {
        let ps: Vec<Placement> = vec![];
        assert!(peak(
            &ps,
            LocationId::new(),
            d(2026, 5, 1),
            d(2026, 5, 1),
            HORIZON()
        )
        .is_none());
    }

    #[test]
    fn peak_ties_pick_the_earliest_date() {
        // Two disjoint equal loads: the earliest start wins the tie.
        let leaf = LocationId::new();
        let root = LocationId::new();
        let a = placed(
            leaf,
            root,
            dec!(10),
            false,
            d(2026, 4, 1),
            Some(d(2026, 5, 1)),
        );
        let b = placed(
            leaf,
            root,
            dec!(10),
            false,
            d(2026, 6, 1),
            Some(d(2026, 7, 1)),
        );
        let ps = vec![a, b];
        let pk = peak(&ps, root, d(2026, 1, 1), d(2026, 12, 1), HORIZON()).unwrap();
        assert_eq!(pk.load.total(), dec!(10));
        assert_eq!(pk.at, d(2026, 4, 1));
    }

    // ---- defensive: absurd values must not panic ---------------------------

    #[test]
    fn absurd_bed_meters_saturate_not_panic() {
        let leaf = LocationId::new();
        let root = LocationId::new();
        let huge = Decimal::MAX;
        let ps = vec![
            placed(leaf, root, huge, false, d(2026, 1, 1), Some(d(2027, 1, 1))),
            placed(leaf, root, huge, false, d(2026, 1, 1), Some(d(2027, 1, 1))),
        ];
        // Two MAX footprints must saturate, not overflow-panic.
        let load = occupancy_at(&ps, root, d(2026, 6, 1), HORIZON());
        assert_eq!(load.open, Decimal::MAX);
        assert_eq!(load.total(), Decimal::MAX);
    }

    #[test]
    fn inverted_interval_is_never_active() {
        let leaf = LocationId::new();
        let root = LocationId::new();
        // end < start: an impossible-but-defensive input.
        let ps = vec![placed(
            leaf,
            root,
            dec!(10),
            false,
            d(2026, 7, 1),
            Some(d(2026, 4, 1)),
        )];
        assert_eq!(
            occupancy_at(&ps, leaf, d(2026, 5, 1), HORIZON()).total(),
            dec!(0)
        );
    }

    // ---- occupancy_window (schedule → interval) ----------------------------

    #[test]
    fn window_of_transplanted_cycle_starts_at_transplant_ends_after_last_harvest() {
        let s = PlantingSchedule::cycle(
            Some(d(2026, 3, 1)),
            Some(d(2026, 5, 1)),
            d(2026, 7, 1),
            d(2026, 10, 1),
        )
        .unwrap();
        let (start, end) = occupancy_window(&s).unwrap();
        assert_eq!(start, d(2026, 5, 1)); // transplant, not sowing
        assert_eq!(end, Some(d(2026, 10, 2))); // last harvest + 1 (inclusive day)
    }

    #[test]
    fn window_of_direct_sown_cycle_starts_at_sowing() {
        let s = PlantingSchedule::cycle(Some(d(2026, 4, 1)), None, d(2026, 7, 1), d(2026, 8, 15))
            .unwrap();
        let (start, _end) = occupancy_window(&s).unwrap();
        assert_eq!(start, d(2026, 4, 1));
    }

    #[test]
    fn window_of_perennial_is_open_ended_without_removal() {
        let s = PlantingSchedule::perennial(d(1996, 3, 1), None).unwrap();
        let (start, end) = occupancy_window(&s).unwrap();
        assert_eq!(start, d(1996, 3, 1));
        assert_eq!(end, None);
    }

    #[test]
    fn window_of_perennial_ends_exclusive_at_removal() {
        let s = PlantingSchedule::perennial(d(2020, 3, 1), Some(d(2040, 11, 1))).unwrap();
        let (start, end) = occupancy_window(&s).unwrap();
        assert_eq!(start, d(2020, 3, 1));
        assert_eq!(end, Some(d(2040, 11, 1))); // free again on the removal day
    }

    // ========================================================================
    // Algebraic property tests (AR13). The engine is a step-function sum over
    // half-open intervals; these pin the algebra a correct implementation owes.
    // ========================================================================

    // Fixed, deterministic hierarchy for the property tests: one root with two
    // beds beneath it. Stable ids let generated placements share a hierarchy so
    // aggregation properties are meaningful.
    fn root_id() -> LocationId {
        LocationId::from(Uuid::from_u128(1))
    }
    fn bed_id(i: u8) -> LocationId {
        LocationId::from(Uuid::from_u128(0x1000 + u128::from(i)))
    }

    /// Day-offset space that includes ±50-year retro-entry, built as offsets
    /// from an epoch so leap years / boundaries are exercised.
    fn epoch() -> NaiveDate {
        NaiveDate::from_ymd_opt(2000, 1, 1).unwrap()
    }

    fn day(offset: i64) -> NaiveDate {
        epoch()
            .checked_add_signed(chrono::Duration::days(offset))
            .unwrap()
    }

    fn off(date: NaiveDate) -> i64 {
        (date - epoch()).num_days()
    }

    prop_compose! {
        /// A bounded-magnitude placement on one of the two beds under `root`.
        fn arb_placement()(
            bed in 0u8..2,
            meters in 0i64..1_000_000,
            covered in any::<bool>(),
            start_off in -20_000i64..20_000, // ~±54 years
            len in 0i64..4_000,              // up to ~11 years
            open_ended in any::<bool>(),
        ) -> Placement {
            let start = day(start_off);
            let end = if open_ended { None } else { Some(day(start_off + len)) };
            Placement {
                path: vec![bed_id(bed), root_id()],
                bed_meters: Decimal::from(meters),
                covered,
                start,
                end,
            }
        }
    }

    fn arb_placements() -> impl Strategy<Value = Vec<Placement>> {
        prop::collection::vec(arb_placement(), 0..12)
    }

    // A far horizon, safely beyond every generated end.
    fn horizon() -> NaiveDate {
        day(30_000)
    }

    proptest! {
        // Superposition / additivity: splitting the placement set and summing
        // the two occupancies equals the occupancy of the union.
        #[test]
        fn prop_superposition(ps in arb_placements(), split_at in 0usize..12, t_off in -25_000i64..25_000) {
            let (root, t, h) = (root_id(), day(t_off), horizon());
            let cut = split_at.min(ps.len());
            let (a, b) = ps.split_at(cut);
            let whole = occupancy_at(&ps, root, t, h);
            let part_a = occupancy_at(a, root, t, h);
            let part_b = occupancy_at(b, root, t, h);
            prop_assert_eq!(whole.covered, part_a.covered.saturating_add(part_b.covered));
            prop_assert_eq!(whole.open, part_a.open.saturating_add(part_b.open));
        }

        // Commutativity: occupancy is independent of placement order.
        #[test]
        fn prop_commutativity(ps in arb_placements(), t_off in -25_000i64..25_000, rot in 0usize..12) {
            let (root, t, h) = (root_id(), day(t_off), horizon());
            let mut rotated = ps.clone();
            let len = rotated.len();
            if len != 0 {
                rotated.rotate_left(rot % len);
            }
            prop_assert_eq!(occupancy_at(&ps, root, t, h), occupancy_at(&rotated, root, t, h));
        }

        // Monotonicity: adding a placement never lowers occupancy anywhere.
        #[test]
        fn prop_monotonicity(ps in arb_placements(), extra in arb_placement(), t_off in -25_000i64..25_000) {
            let (root, t, h) = (root_id(), day(t_off), horizon());
            let before = occupancy_at(&ps, root, t, h);
            let mut more = ps.clone();
            more.push(extra);
            let after = occupancy_at(&more, root, t, h);
            prop_assert!(after.covered >= before.covered);
            prop_assert!(after.open >= before.open);
        }

        // Translation invariance: shifting every interval AND the query instant
        // by the same offset yields the identical occupancy.
        #[test]
        fn prop_translation_invariance(ps in arb_placements(), t_off in -20_000i64..20_000, shift in -5_000i64..5_000) {
            let root = root_id();
            let h_off = 30_000i64;
            let shifted: Vec<Placement> = ps.iter().map(|p| Placement {
                path: p.path.clone(),
                bed_meters: p.bed_meters,
                covered: p.covered,
                start: day(off(p.start) + shift),
                end: p.end.map(|e| day(off(e) + shift)),
            }).collect();

            let base = occupancy_at(&ps, root, day(t_off), day(h_off));
            // Shift the horizon too so open-ended placements translate consistently.
            let moved = occupancy_at(&shifted, root, day(t_off + shift), day(h_off + shift));
            prop_assert_eq!(base, moved);
        }

        // Adjacent-non-overlap: for [a,b)+[b,c) on one bed, the load equals
        // exactly one footprint (never their sum); at the shared boundary it is
        // the SECOND.
        #[test]
        fn prop_adjacent_non_overlap(
            a_start in -10_000i64..10_000,
            len_a in 1i64..2_000,
            len_b in 1i64..2_000,
            m1 in 1i64..100_000,
            m2 in 1i64..100_000,
        ) {
            let (leaf, root) = (bed_id(0), root_id());
            let b_start = a_start + len_a;
            let c = b_start + len_b;
            let first = placed(leaf, root, Decimal::from(m1), false, day(a_start), Some(day(b_start)));
            let second = placed(leaf, root, Decimal::from(m2), false, day(b_start), Some(day(c)));
            let ps = vec![first, second];
            let h = horizon();
            // At the boundary: only the second is active.
            prop_assert_eq!(occupancy_at(&ps, leaf, day(b_start), h).total(), Decimal::from(m2));
            // Mid-first: only the first.
            prop_assert_eq!(occupancy_at(&ps, leaf, day(a_start), h).total(), Decimal::from(m1));
        }

        // Horizon-extension stability: for bounded placements, occupancy at any
        // instant is independent of the horizon.
        #[test]
        fn prop_horizon_extension_stability(ps in arb_placements(), t_off in -20_000i64..20_000) {
            let root = root_id();
            // Force bounded ends by mapping open-ended to a fixed far end.
            let bounded: Vec<Placement> = ps.into_iter()
                .map(|p| Placement { end: Some(p.end.unwrap_or(day(25_000))), ..p })
                .collect();
            let t = day(t_off);
            prop_assert_eq!(
                occupancy_at(&bounded, root, t, day(26_000)),
                occupancy_at(&bounded, root, t, day(60_000))
            );
        }

        // Hierarchy coherence: at every sampled instant, the root load equals
        // the sum of its two beds' loads.
        #[test]
        fn prop_hierarchy_coherence(ps in arb_placements(), t_off in -25_000i64..25_000) {
            let (root, t, h) = (root_id(), day(t_off), horizon());
            let at_root = occupancy_at(&ps, root, t, h);
            let at_b0 = occupancy_at(&ps, bed_id(0), t, h);
            let at_b1 = occupancy_at(&ps, bed_id(1), t, h);
            prop_assert_eq!(at_root.covered, at_b0.covered.saturating_add(at_b1.covered));
            prop_assert_eq!(at_root.open, at_b0.open.saturating_add(at_b1.open));
        }

        // ±50-year retro-entry: intervals starting decades in the past compute
        // without panic, and a placement spanning `t` is counted.
        #[test]
        fn prop_retro_entry_no_panic(meters in 0i64..1_000_000, span in 1i64..30_000) {
            let (leaf, root) = (bed_id(0), root_id());
            // Start ~50 years before the epoch.
            let start = day(-18_250);
            let ps = vec![placed(leaf, root, Decimal::from(meters), false, start, Some(day(-18_250 + span)))];
            let h = day(40_000);
            // Sampling anywhere never panics.
            let _ = occupancy_at(&ps, root, day(-18_250), h);
            let _ = occupancy_at(&ps, root, day(-18_250 + span), h);
            // A point strictly inside is counted.
            if span > 1 {
                prop_assert_eq!(
                    occupancy_at(&ps, root, day(-18_250 + span - 1), h).total(),
                    Decimal::from(meters)
                );
            }
        }
    }
}
