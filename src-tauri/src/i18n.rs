//! Translation lookup shared by the tray, native dialogs, and window titles.
//! The webviews load the same `locales/*.json` catalogs through Vite; keeping
//! one file per language means Rust and TypeScript can never disagree.

use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

/// Supported UI languages. `EnGb` is the fallback for everything unknown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Lang {
    EnGb,
    Es,
    Fr,
    De,
    It,
}

impl Lang {
    /// Match a BCP-47-ish tag ("es", "es-MX", "fr_FR") by its primary subtag;
    /// anything unsupported falls back to en-GB.
    pub fn from_tag(tag: &str) -> Self {
        let primary = tag
            .split(['-', '_'])
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        match primary.as_str() {
            "es" => Lang::Es,
            "fr" => Lang::Fr,
            "de" => Lang::De,
            "it" => Lang::It,
            _ => Lang::EnGb,
        }
    }

    /// The tag the frontend uses to pick its catalog (matches locales/*.json).
    pub fn tag(self) -> &'static str {
        match self {
            Lang::EnGb => "en-GB",
            Lang::Es => "es",
            Lang::Fr => "fr",
            Lang::De => "de",
            Lang::It => "it",
        }
    }
}

static CURRENT: RwLock<Lang> = RwLock::new(Lang::EnGb);
static CATALOGS: OnceLock<HashMap<Lang, HashMap<String, String>>> = OnceLock::new();

fn catalogs() -> &'static HashMap<Lang, HashMap<String, String>> {
    CATALOGS.get_or_init(|| {
        [
            (Lang::EnGb, include_str!("../../locales/en-GB.json")),
            (Lang::Es, include_str!("../../locales/es.json")),
            (Lang::Fr, include_str!("../../locales/fr.json")),
            (Lang::De, include_str!("../../locales/de.json")),
            (Lang::It, include_str!("../../locales/it.json")),
        ]
        .into_iter()
        .map(|(lang, json)| {
            (
                lang,
                serde_json::from_str(json).expect("locale catalogs are valid flat JSON"),
            )
        })
        .collect()
    })
}

/// Resolve a settings value ("system" or an explicit tag) to a language.
pub fn resolve(setting: &str) -> Lang {
    if setting == "system" {
        sys_locale::get_locale()
            .map(|tag| Lang::from_tag(&tag))
            .unwrap_or(Lang::EnGb)
    } else {
        Lang::from_tag(setting)
    }
}

pub fn set_language(lang: Lang) {
    *CURRENT.write().unwrap() = lang;
}

pub fn current() -> Lang {
    *CURRENT.read().unwrap()
}

/// Translated string for `key`: current language → en-GB → the key itself.
pub fn t(key: &str) -> String {
    let catalogs = catalogs();
    catalogs
        .get(&current())
        .and_then(|c| c.get(key))
        .or_else(|| catalogs.get(&Lang::EnGb).and_then(|c| c.get(key)))
        .cloned()
        .unwrap_or_else(|| key.to_string())
}

/// `t` plus `{name}` placeholder replacement (no plural rules — the only
/// plural string lives frontend-side).
pub fn t_with(key: &str, args: &[(&str, &str)]) -> String {
    let mut text = t(key);
    for (name, value) in args {
        text = text.replace(&format!("{{{name}}}"), value);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_tag_matches_primary_subtag() {
        assert_eq!(Lang::from_tag("es-ES"), Lang::Es);
        assert_eq!(Lang::from_tag("es_MX"), Lang::Es);
        assert_eq!(Lang::from_tag("fr"), Lang::Fr);
        assert_eq!(Lang::from_tag("DE-at"), Lang::De);
        assert_eq!(Lang::from_tag("it-IT"), Lang::It);
        assert_eq!(Lang::from_tag("en-US"), Lang::EnGb);
        assert_eq!(Lang::from_tag("pt-BR"), Lang::EnGb);
        assert_eq!(Lang::from_tag(""), Lang::EnGb);
        assert_eq!(Lang::from_tag("garbage"), Lang::EnGb);
    }

    #[test]
    fn resolve_explicit_tags() {
        assert_eq!(resolve("es"), Lang::Es);
        assert_eq!(resolve("en-GB"), Lang::EnGb);
        assert_eq!(resolve("zz"), Lang::EnGb);
    }

    #[test]
    fn catalogs_parse_and_share_key_set() {
        let catalogs = catalogs();
        let reference: std::collections::BTreeSet<_> =
            catalogs[&Lang::EnGb].keys().collect();
        for lang in [Lang::Es, Lang::Fr, Lang::De, Lang::It] {
            let keys: std::collections::BTreeSet<_> = catalogs[&lang].keys().collect();
            assert_eq!(keys, reference, "key set mismatch for {lang:?}");
        }
    }

    // Single test: `CURRENT` is process-global and cargo runs tests in
    // parallel, so language switches must not be split across tests.
    #[test]
    fn lookup_fallback_and_interpolation() {
        set_language(Lang::Es);
        assert_eq!(t("tray.settings"), "Ajustes…");
        assert_eq!(t("no.such.key"), "no.such.key");
        set_language(Lang::EnGb);
        assert_eq!(t("tray.settings"), "Settings…");
        let text = t_with("updates.available", &[("version", "2.0.0")]);
        assert!(text.contains("Version 2.0.0"), "{text}");
    }
}
