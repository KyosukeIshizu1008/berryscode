//! Internationalization (i18n) module
//!
//! Provides a `t()` macro-like function to translate UI strings
//! based on the current UiLanguage setting.

use super::types::UiLanguage;
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct LangFile {
    #[serde(flatten)]
    pub entries: HashMap<String, String>,
}

static LANG_CACHE: Lazy<HashMap<&'static str, HashMap<&'static str, &'static str>>> =
    Lazy::new(|| {
        let locales = [
            ("ja_JP", include_str!("../../assets/i18n/ja_JP.toml")),
            ("ko_KR", include_str!("../../assets/i18n/ko_KR.toml")),
        ];

        locales
            .into_iter()
            .map(|(locale, content)| {
                let parsed = toml::from_str::<LangFile>(content)
                    .unwrap_or_else(|e| panic!("Failed to parse locale file ({locale}): {e}"));

                let entries = parsed
                    .entries
                    .into_iter()
                    .map(|(k, v)| {
                        let key: &'static str = Box::leak(k.into_boxed_str());
                        let value: &'static str = Box::leak(v.into_boxed_str());

                        (key, value)
                    })
                    .collect::<HashMap<_, _>>();

                (locale, entries)
            })
            .collect()
    });

/// Translate a UI string based on the current language.
/// English is the source-of-truth key.
pub fn t(lang: UiLanguage, en: &'static str) -> &'static str {
    if matches!(lang, UiLanguage::English) {
        return en;
    }

    LANG_CACHE
        .get(lang.locale())
        .and_then(|file| file.get(en))
        .copied()
        .unwrap_or(en)
}
