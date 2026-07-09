//! Public-holiday calculation per region (issue #35).
//!
//! Holidays are **computed from rules**, not stored: fixed dates, the
//! Easter-derived block (Gregorian Easter via the Meeus/Jones/Butcher
//! algorithm) and the two Swiss "Jeûne" specials. This works for any year
//! with nothing to seed or migrate; the configured [`HolidayRegion`] lives
//! in the app config, not the database.
//!
//! Regions are the French-speaking Swiss cantons plus France — Pomone's
//! current audience. The per-region sets list the *legal* public holidays;
//! locally-observed extras (e.g. patron-saint days, half-days) are out of
//! scope, as are conditional replacement days (2 Jan when 1 Jan falls on a
//! Sunday in some cantons).

use chrono::{Datelike, Days, NaiveDate, Weekday};

/// A supported holiday region. `code()`/`parse()` round-trip the stable
/// string persisted in the app config (e.g. `"ch-vd"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HolidayRegion {
    /// Vaud
    ChVd,
    /// Genève
    ChGe,
    /// Neuchâtel
    ChNe,
    /// Fribourg
    ChFr,
    /// Valais
    ChVs,
    /// Jura
    ChJu,
    /// France (métropole)
    Fr,
}

impl HolidayRegion {
    /// Every supported region, in the order the settings picker lists them.
    pub const ALL: [Self; 7] = [
        Self::ChVd,
        Self::ChGe,
        Self::ChNe,
        Self::ChFr,
        Self::ChVs,
        Self::ChJu,
        Self::Fr,
    ];

    /// Stable config/persistence code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::ChVd => "ch-vd",
            Self::ChGe => "ch-ge",
            Self::ChNe => "ch-ne",
            Self::ChFr => "ch-fr",
            Self::ChVs => "ch-vs",
            Self::ChJu => "ch-ju",
            Self::Fr => "fr",
        }
    }

    /// Fluent key suffix for the region's display name
    /// (`holiday-region-<key>`).
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::ChVd => "ch-vd",
            Self::ChGe => "ch-ge",
            Self::ChNe => "ch-ne",
            Self::ChFr => "ch-fr",
            Self::ChVs => "ch-vs",
            Self::ChJu => "ch-ju",
            Self::Fr => "fr",
        }
    }

    /// Parse a persisted code; `None` for unknown codes (including the
    /// empty string, which the config uses for "no region / feature off").
    #[must_use]
    pub fn parse(code: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|r| r.code() == code)
    }
}

/// One public holiday, identified independently of its date so the UI can
/// localize the label (`holiday-<key>` Fluent keys).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Holiday {
    /// 1 January.
    NewYear,
    /// 2 January (Saint Berchtold).
    BerchtoldDay,
    /// 1 March — Instauration de la République (Neuchâtel).
    NeuchatelRepublic,
    /// 19 March — Saint Joseph (Valais).
    StJoseph,
    /// Easter − 2 days.
    GoodFriday,
    /// Easter + 1 day.
    EasterMonday,
    /// 1 May.
    LabourDay,
    /// 8 May — Victoire 1945 (France).
    VictoryDay,
    /// Easter + 39 days.
    Ascension,
    /// Easter + 50 days.
    WhitMonday,
    /// Easter + 60 days — Fête-Dieu.
    CorpusChristi,
    /// 23 June — Indépendance jurassienne.
    JuraIndependence,
    /// 14 July (France).
    BastilleDay,
    /// 1 August (Switzerland).
    SwissNationalDay,
    /// 15 August — Assomption.
    Assumption,
    /// Thursday after the first Sunday of September — Jeûne genevois.
    GenevaFast,
    /// Monday after the third Sunday of September — lundi du Jeûne fédéral.
    FederalFastMonday,
    /// 1 November — Toussaint.
    AllSaints,
    /// 11 November — Armistice 1918 (France).
    ArmisticeDay,
    /// 8 December — Immaculée Conception.
    ImmaculateConception,
    /// 25 December.
    Christmas,
    /// 26 December — Saint Étienne.
    StStephensDay,
    /// 31 December — Restauration de la République (Genève).
    GenevaRestoration,
}

impl Holiday {
    /// Fluent key suffix for the localized label (`holiday-<key>`).
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::NewYear => "new-year",
            Self::BerchtoldDay => "berchtold",
            Self::NeuchatelRepublic => "neuchatel-republic",
            Self::StJoseph => "st-joseph",
            Self::GoodFriday => "good-friday",
            Self::EasterMonday => "easter-monday",
            Self::LabourDay => "labour-day",
            Self::VictoryDay => "victory-day",
            Self::Ascension => "ascension",
            Self::WhitMonday => "whit-monday",
            Self::CorpusChristi => "corpus-christi",
            Self::JuraIndependence => "jura-independence",
            Self::BastilleDay => "bastille-day",
            Self::SwissNationalDay => "swiss-national-day",
            Self::Assumption => "assumption",
            Self::GenevaFast => "geneva-fast",
            Self::FederalFastMonday => "federal-fast-monday",
            Self::AllSaints => "all-saints",
            Self::ArmisticeDay => "armistice-day",
            Self::ImmaculateConception => "immaculate-conception",
            Self::Christmas => "christmas",
            Self::StStephensDay => "st-stephens",
            Self::GenevaRestoration => "geneva-restoration",
        }
    }
}

/// Gregorian Easter Sunday (Meeus/Jones/Butcher). `None` outside chrono's
/// date range — callers just get an empty holiday list for such years.
#[must_use]
pub fn easter_sunday(year: i32) -> Option<NaiveDate> {
    let a = year.rem_euclid(19);
    let b = year.div_euclid(100);
    let c = year.rem_euclid(100);
    let d = b / 4;
    let e = b % 4;
    let f = (b + 8) / 25;
    let g = (b - f + 1) / 3;
    let h = (19 * a + b - d - g + 15) % 30;
    let i = c / 4;
    let k = c % 4;
    let l = (32 + 2 * e + 2 * i - h - k) % 7;
    let m = (a + 11 * h + 22 * l) / 451;
    let month = (h + l - 7 * m + 114) / 31;
    let day = ((h + l - 7 * m + 114) % 31) + 1;
    #[allow(clippy::cast_sign_loss)]
    NaiveDate::from_ymd_opt(year, month as u32, day as u32)
}

/// First `weekday` on or after `date`.
fn next_weekday_on_or_after(date: NaiveDate, weekday: Weekday) -> Option<NaiveDate> {
    let delta = (7 + weekday.num_days_from_monday() - date.weekday().num_days_from_monday()) % 7;
    date.checked_add_days(Days::new(u64::from(delta)))
}

/// Monday after the third Sunday of September (lundi du Jeûne fédéral).
fn federal_fast_monday(year: i32) -> Option<NaiveDate> {
    let first_sunday =
        next_weekday_on_or_after(NaiveDate::from_ymd_opt(year, 9, 1)?, Weekday::Sun)?;
    first_sunday.checked_add_days(Days::new(15))
}

/// Thursday after the first Sunday of September (Jeûne genevois).
fn geneva_fast(year: i32) -> Option<NaiveDate> {
    let first_sunday =
        next_weekday_on_or_after(NaiveDate::from_ymd_opt(year, 9, 1)?, Weekday::Sun)?;
    first_sunday.checked_add_days(Days::new(4))
}

/// The legal public holidays of `region` for `year`, sorted by date.
#[must_use]
pub fn holidays_in_year(region: HolidayRegion, year: i32) -> Vec<(NaiveDate, Holiday)> {
    use Holiday as H;
    use HolidayRegion as R;

    let fixed = |m: u32, d: u32| NaiveDate::from_ymd_opt(year, m, d);
    let easter = easter_sunday(year);
    let rel = |days: u64| easter.and_then(|e| e.checked_add_days(Days::new(days)));
    let before = |days: u64| easter.and_then(|e| e.checked_sub_days(Days::new(days)));

    let candidates: Vec<(Option<NaiveDate>, H)> = match region {
        R::ChVd => vec![
            (fixed(1, 1), H::NewYear),
            (fixed(1, 2), H::BerchtoldDay),
            (before(2), H::GoodFriday),
            (rel(1), H::EasterMonday),
            (rel(39), H::Ascension),
            (rel(50), H::WhitMonday),
            (fixed(8, 1), H::SwissNationalDay),
            (federal_fast_monday(year), H::FederalFastMonday),
            (fixed(12, 25), H::Christmas),
        ],
        R::ChGe => vec![
            (fixed(1, 1), H::NewYear),
            (before(2), H::GoodFriday),
            (rel(1), H::EasterMonday),
            (rel(39), H::Ascension),
            (rel(50), H::WhitMonday),
            (fixed(8, 1), H::SwissNationalDay),
            (geneva_fast(year), H::GenevaFast),
            (fixed(12, 25), H::Christmas),
            (fixed(12, 31), H::GenevaRestoration),
        ],
        R::ChNe => vec![
            (fixed(1, 1), H::NewYear),
            (fixed(3, 1), H::NeuchatelRepublic),
            (before(2), H::GoodFriday),
            (rel(1), H::EasterMonday),
            (fixed(5, 1), H::LabourDay),
            (rel(39), H::Ascension),
            (rel(50), H::WhitMonday),
            (fixed(8, 1), H::SwissNationalDay),
            (fixed(12, 25), H::Christmas),
        ],
        R::ChFr => vec![
            (fixed(1, 1), H::NewYear),
            (before(2), H::GoodFriday),
            (rel(1), H::EasterMonday),
            (rel(39), H::Ascension),
            (rel(50), H::WhitMonday),
            (rel(60), H::CorpusChristi),
            (fixed(8, 1), H::SwissNationalDay),
            (fixed(8, 15), H::Assumption),
            (fixed(11, 1), H::AllSaints),
            (fixed(12, 8), H::ImmaculateConception),
            (fixed(12, 25), H::Christmas),
            (fixed(12, 26), H::StStephensDay),
        ],
        R::ChVs => vec![
            (fixed(1, 1), H::NewYear),
            (fixed(3, 19), H::StJoseph),
            (rel(39), H::Ascension),
            (rel(60), H::CorpusChristi),
            (fixed(8, 1), H::SwissNationalDay),
            (fixed(8, 15), H::Assumption),
            (fixed(11, 1), H::AllSaints),
            (fixed(12, 8), H::ImmaculateConception),
            (fixed(12, 25), H::Christmas),
        ],
        R::ChJu => vec![
            (fixed(1, 1), H::NewYear),
            (fixed(1, 2), H::BerchtoldDay),
            (before(2), H::GoodFriday),
            (rel(1), H::EasterMonday),
            (fixed(5, 1), H::LabourDay),
            (rel(39), H::Ascension),
            (rel(50), H::WhitMonday),
            (rel(60), H::CorpusChristi),
            (fixed(6, 23), H::JuraIndependence),
            (fixed(8, 1), H::SwissNationalDay),
            (fixed(8, 15), H::Assumption),
            (fixed(11, 1), H::AllSaints),
            (fixed(12, 25), H::Christmas),
        ],
        R::Fr => vec![
            (fixed(1, 1), H::NewYear),
            (rel(1), H::EasterMonday),
            (fixed(5, 1), H::LabourDay),
            (fixed(5, 8), H::VictoryDay),
            (rel(39), H::Ascension),
            (rel(50), H::WhitMonday),
            (fixed(7, 14), H::BastilleDay),
            (fixed(8, 15), H::Assumption),
            (fixed(11, 1), H::AllSaints),
            (fixed(11, 11), H::ArmisticeDay),
            (fixed(12, 25), H::Christmas),
        ],
    };

    let mut out: Vec<(NaiveDate, Holiday)> = candidates
        .into_iter()
        .filter_map(|(date, h)| date.map(|d| (d, h)))
        .collect();
    out.sort_by_key(|(d, _)| *d);
    out
}

/// The holiday falling on `date` in `region`, if any.
#[must_use]
pub fn holiday_on(region: HolidayRegion, date: NaiveDate) -> Option<Holiday> {
    holidays_in_year(region, date.year())
        .into_iter()
        .find_map(|(d, h)| (d == date).then_some(h))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn easter_matches_known_years() {
        // Reference dates from published tables.
        assert_eq!(easter_sunday(2024), Some(d(2024, 3, 31)));
        assert_eq!(easter_sunday(2025), Some(d(2025, 4, 20)));
        assert_eq!(easter_sunday(2026), Some(d(2026, 4, 5)));
        assert_eq!(easter_sunday(2027), Some(d(2027, 3, 28)));
        assert_eq!(easter_sunday(2030), Some(d(2030, 4, 21)));
    }

    #[test]
    fn vaud_2026_holidays_are_correct() {
        let hs = holidays_in_year(HolidayRegion::ChVd, 2026);
        let dates: Vec<_> = hs.iter().map(|(d, _)| *d).collect();
        assert_eq!(
            dates,
            vec![
                d(2026, 1, 1),   // Nouvel an
                d(2026, 1, 2),   // Berchtold
                d(2026, 4, 3),   // Vendredi saint (Pâques 5 avril)
                d(2026, 4, 6),   // Lundi de Pâques
                d(2026, 5, 14),  // Ascension
                d(2026, 5, 25),  // Lundi de Pentecôte
                d(2026, 8, 1),   // Fête nationale
                d(2026, 9, 21),  // Lundi du Jeûne fédéral
                d(2026, 12, 25), // Noël
            ]
        );
    }

    #[test]
    fn federal_fast_is_monday_after_third_september_sunday() {
        // 2026: Sundays in September are 6, 13, 20, 27 → fast Monday = 21.
        assert_eq!(federal_fast_monday(2026), Some(d(2026, 9, 21)));
        // 2024: Sundays are 1, 8, 15, 22 → fast Monday = 16.
        assert_eq!(federal_fast_monday(2024), Some(d(2024, 9, 16)));
        assert_eq!(federal_fast_monday(2026).unwrap().weekday(), Weekday::Mon);
    }

    #[test]
    fn geneva_fast_is_thursday_after_first_september_sunday() {
        // 2026: first September Sunday is the 6th → Thursday the 10th.
        assert_eq!(geneva_fast(2026), Some(d(2026, 9, 10)));
        assert_eq!(geneva_fast(2026).unwrap().weekday(), Weekday::Thu);
    }

    #[test]
    fn holiday_on_finds_and_misses() {
        assert_eq!(
            holiday_on(HolidayRegion::ChGe, d(2026, 12, 31)),
            Some(Holiday::GenevaRestoration)
        );
        // Geneva has no 2 January holiday.
        assert_eq!(holiday_on(HolidayRegion::ChGe, d(2026, 1, 2)), None);
        // France: 14 July.
        assert_eq!(
            holiday_on(HolidayRegion::Fr, d(2026, 7, 14)),
            Some(Holiday::BastilleDay)
        );
    }

    #[test]
    fn valais_skips_easter_monday_but_has_corpus_christi() {
        assert_eq!(holiday_on(HolidayRegion::ChVs, d(2026, 4, 6)), None);
        // Fête-Dieu 2026 = Pâques (5 avril) + 60 jours = 4 juin.
        assert_eq!(
            holiday_on(HolidayRegion::ChVs, d(2026, 6, 4)),
            Some(Holiday::CorpusChristi)
        );
    }

    #[test]
    fn region_codes_roundtrip() {
        for r in HolidayRegion::ALL {
            assert_eq!(HolidayRegion::parse(r.code()), Some(r));
        }
        assert_eq!(HolidayRegion::parse(""), None);
        assert_eq!(HolidayRegion::parse("mars"), None);
    }

    #[test]
    fn holidays_are_sorted_and_within_year() {
        for r in HolidayRegion::ALL {
            let hs = holidays_in_year(r, 2027);
            let dates: Vec<_> = hs.iter().map(|(d, _)| *d).collect();
            let mut sorted = dates.clone();
            sorted.sort_unstable();
            assert_eq!(dates, sorted, "{r:?} must be sorted");
            assert!(dates.iter().all(|d| d.year() == 2027));
        }
    }
}
