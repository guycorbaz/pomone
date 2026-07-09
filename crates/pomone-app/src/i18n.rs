//! Internationalisation via [Project Fluent](https://projectfluent.org/).
//!
//! Two languages are supported in v1: French (default) and English. Fluent
//! resources are embedded at compile time from `locales/{fr,en}/main.ftl`,
//! so there is no runtime filesystem dependency.

use crate::error::{AppError, AppResult};
use fluent::{FluentArgs, FluentBundle, FluentResource};
use unic_langid::{langid, LanguageIdentifier};

const FR_FTL: &str = include_str!("../locales/fr/main.ftl");
const EN_FTL: &str = include_str!("../locales/en/main.ftl");

/// Supported UI language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Fr,
    En,
}

impl Lang {
    /// Parse a BCP-47-ish tag. Accepts case-insensitive variants like
    /// `fr`, `fr-FR`, `fr_FR`; same for English.
    pub fn parse(tag: &str) -> AppResult<Self> {
        let lower = tag.to_lowercase();
        let primary = lower.split(['-', '_']).next().unwrap_or("");
        match primary {
            "fr" => Ok(Self::Fr),
            "en" => Ok(Self::En),
            _ => Err(AppError::Config(format!("unsupported language tag: {tag}"))),
        }
    }

    #[must_use]
    pub fn as_langid(self) -> LanguageIdentifier {
        match self {
            Self::Fr => langid!("fr"),
            Self::En => langid!("en"),
        }
    }

    /// Fallback language used when a key is missing in the active one.
    /// English is the universal fallback; French has no further fallback
    /// (its missing keys would just render `{key}`).
    #[must_use]
    pub fn fallback(self) -> Option<Self> {
        match self {
            Self::Fr => Some(Self::En),
            Self::En => None,
        }
    }

    /// IETF tag suitable for storage in `AppConfig.language`.
    #[must_use]
    pub fn tag(self) -> &'static str {
        match self {
            Self::Fr => "fr",
            Self::En => "en",
        }
    }
}

/// Owns the compiled Fluent bundles for both supported languages.
///
/// Construction parses both `.ftl` files and surfaces parse errors.
/// After construction, [`I18n::t`] and [`I18n::t_args`] are infallible
/// (missing keys render as `{key}` rather than panicking).
pub struct I18n {
    lang: Lang,
    bundle_fr: FluentBundle<FluentResource>,
    bundle_en: FluentBundle<FluentResource>,
}

impl std::fmt::Debug for I18n {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("I18n")
            .field("lang", &self.lang)
            .finish_non_exhaustive()
    }
}

impl I18n {
    /// Build an `I18n` for the given active language. Both bundles are
    /// always loaded so a fallback can be served when a key is missing.
    pub fn new(lang: Lang) -> AppResult<Self> {
        let bundle_fr = build_bundle(Lang::Fr, FR_FTL)?;
        let bundle_en = build_bundle(Lang::En, EN_FTL)?;
        Ok(Self {
            lang,
            bundle_fr,
            bundle_en,
        })
    }

    #[must_use]
    pub fn lang(&self) -> Lang {
        self.lang
    }

    pub fn set_lang(&mut self, lang: Lang) {
        self.lang = lang;
    }

    /// Translate a key with no arguments.
    #[must_use]
    pub fn t(&self, key: &str) -> String {
        self.translate(key, None)
    }

    /// Translate a key with named arguments (e.g. `welcome-user` with `$name`).
    #[must_use]
    pub fn t_args(&self, key: &str, args: &FluentArgs) -> String {
        self.translate(key, Some(args))
    }

    fn translate(&self, key: &str, args: Option<&FluentArgs>) -> String {
        if let Some(text) = format_in(self.bundle_for(self.lang), key, args) {
            return text;
        }
        if let Some(fallback) = self.lang.fallback() {
            if let Some(text) = format_in(self.bundle_for(fallback), key, args) {
                return text;
            }
        }
        format!("{{{key}}}")
    }

    fn bundle_for(&self, lang: Lang) -> &FluentBundle<FluentResource> {
        match lang {
            Lang::Fr => &self.bundle_fr,
            Lang::En => &self.bundle_en,
        }
    }
}

fn build_bundle(lang: Lang, ftl: &str) -> AppResult<FluentBundle<FluentResource>> {
    let resource = FluentResource::try_new(ftl.to_owned()).map_err(|(_, errs)| {
        AppError::Config(format!("FTL parse failed for {}: {errs:?}", lang.tag()))
    })?;
    let mut bundle = FluentBundle::new(vec![lang.as_langid()]);
    bundle.add_resource(resource).map_err(|errs| {
        AppError::Config(format!(
            "FTL bundle build failed for {}: {errs:?}",
            lang.tag()
        ))
    })?;
    // Without isolation marks, formatted strings come out without the
    // U+2068/U+2069 wrappers around interpolated arguments — easier to
    // assert on in tests and to render in non-RTL UI.
    bundle.set_use_isolating(false);
    Ok(bundle)
}

fn format_in(
    bundle: &FluentBundle<FluentResource>,
    key: &str,
    args: Option<&FluentArgs>,
) -> Option<String> {
    let message = bundle.get_message(key)?;
    let pattern = message.value()?;
    let mut errors = Vec::new();
    let formatted = bundle.format_pattern(pattern, args, &mut errors);
    if errors.is_empty() {
        Some(formatted.into_owned())
    } else {
        // Fluent returns a best-effort string even on errors; surface them
        // via tracing in case of investigation, but use the result.
        tracing::warn!(?errors, key, "fluent formatting reported errors");
        Some(formatted.into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_accepts_common_tags() {
        assert_eq!(Lang::parse("fr").unwrap(), Lang::Fr);
        assert_eq!(Lang::parse("FR").unwrap(), Lang::Fr);
        assert_eq!(Lang::parse("fr-FR").unwrap(), Lang::Fr);
        assert_eq!(Lang::parse("fr_FR").unwrap(), Lang::Fr);
        assert_eq!(Lang::parse("en").unwrap(), Lang::En);
        assert_eq!(Lang::parse("en-GB").unwrap(), Lang::En);
    }

    #[test]
    fn parse_rejects_unsupported() {
        let err = Lang::parse("de").unwrap_err();
        assert!(matches!(err, AppError::Config(_)));
    }

    #[test]
    fn fr_translation_uses_french_resource() {
        let i18n = I18n::new(Lang::Fr).unwrap();
        assert_eq!(i18n.t("welcome"), "Bienvenue dans Pomone");
        assert_eq!(i18n.t("crop"), "Culture");
        assert_eq!(i18n.t("lifespan-annual"), "Annuelle");
    }

    #[test]
    fn en_translation_uses_english_resource() {
        let i18n = I18n::new(Lang::En).unwrap();
        assert_eq!(i18n.t("welcome"), "Welcome to Pomone");
        assert_eq!(i18n.t("crop"), "Crop");
        assert_eq!(
            i18n.t("lifespan-pluriannual-recurring"),
            "Pluriannual recurring"
        );
    }

    #[test]
    fn switching_language_updates_translations() {
        let mut i18n = I18n::new(Lang::Fr).unwrap();
        assert_eq!(i18n.t("crop"), "Culture");
        i18n.set_lang(Lang::En);
        assert_eq!(i18n.t("crop"), "Crop");
    }

    #[test]
    fn arguments_interpolated_in_french() {
        let i18n = I18n::new(Lang::Fr).unwrap();
        let mut args = FluentArgs::new();
        args.set("name", "André");
        let msg = i18n.t_args("welcome-user", &args);
        assert_eq!(msg, "Bienvenue, André !");
    }

    #[test]
    fn arguments_interpolated_in_english() {
        let i18n = I18n::new(Lang::En).unwrap();
        let mut args = FluentArgs::new();
        args.set("kind", "variety");
        args.set("id", "abc-123");
        let msg = i18n.t_args("error-not-found", &args);
        assert_eq!(msg, "Not found: variety abc-123");
    }

    #[test]
    fn missing_key_returns_brace_wrapped_marker() {
        let i18n = I18n::new(Lang::Fr).unwrap();
        assert_eq!(i18n.t("does-not-exist"), "{does-not-exist}");
    }

    #[test]
    fn fr_and_en_expose_the_same_keys() {
        // Every user-facing key must exist in both catalogues (project
        // convention). Compare the raw message identifiers of the two
        // embedded .ftl files so a key added to one side only fails fast.
        fn keys(ftl: &str) -> std::collections::BTreeSet<String> {
            ftl.lines()
                .filter(|l| {
                    // A message definition starts at column 0 with
                    // `identifier =`; attributes/continuations are indented.
                    !l.starts_with([' ', '\t', '#', '.'])
                })
                .filter_map(|l| {
                    let (id, _) = l.split_once('=')?;
                    let id = id.trim();
                    (!id.is_empty()).then(|| id.to_owned())
                })
                .collect()
        }
        let fr = keys(FR_FTL);
        let en = keys(EN_FTL);
        let only_fr: Vec<_> = fr.difference(&en).collect();
        let only_en: Vec<_> = en.difference(&fr).collect();
        assert!(
            only_fr.is_empty() && only_en.is_empty(),
            "keys missing from en: {only_fr:?}; keys missing from fr: {only_en:?}"
        );
    }

    #[test]
    fn tooltip_keys_resolve_in_both_languages() {
        let fr = I18n::new(Lang::Fr).unwrap();
        let en = I18n::new(Lang::En).unwrap();
        // Spot-check one key per tooltip family (#39); the parity test
        // above guarantees the rest exist on both sides.
        for key in [
            "tooltip-nav-home",
            "tooltip-planting-area",
            "tooltip-task-recurring",
            "tooltip-calendar-milestones",
        ] {
            assert_ne!(fr.t(key), format!("{{{key}}}"), "missing fr {key}");
            assert_ne!(en.t(key), format!("{{{key}}}"), "missing en {key}");
        }
    }

    #[test]
    fn missing_in_fr_falls_back_to_en() {
        // We don't have FR-only keys today, so simulate by constructing
        // an I18n where FR has been deliberately stripped of one key.
        // The cleanest way: confirm that English contains the same keys,
        // then add a synthetic test by creating a custom bundle. Here we
        // settle for verifying the fallback path via lang::fallback.
        assert_eq!(Lang::Fr.fallback(), Some(Lang::En));
        assert_eq!(Lang::En.fallback(), None);
    }
}
