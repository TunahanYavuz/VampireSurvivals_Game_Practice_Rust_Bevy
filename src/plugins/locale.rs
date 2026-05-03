use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Supported UI languages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Language {
    #[default]
    English,
    Turkish,
}

impl Language {
    pub fn code(&self) -> &str {
        match self {
            Language::English => "en",
            Language::Turkish => "tr",
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            Language::English => "English",
            Language::Turkish => "Türkçe",
        }
    }

    pub fn toggle(&self) -> Self {
        match self {
            Language::English => Language::Turkish,
            Language::Turkish => Language::English,
        }
    }
}

/// Holds the active language and its loaded translation strings.
#[derive(Resource, Clone)]
pub struct Locale {
    pub language: Language,
    translations: HashMap<String, String>,
}

impl Locale {
    /// Load translation strings from `assets/locales/<code>.ron`.
    pub fn load(language: Language) -> Self {
        let path = format!("assets/locales/{}.ron", language.code());
        let translations = match std::fs::read_to_string(&path) {
            Ok(content) => ron::from_str::<HashMap<String, String>>(&content).unwrap_or_default(),
            Err(e) => {
                eprintln!("Failed to load locale '{}': {e}", language.code());
                HashMap::new()
            }
        };
        Self {
            language,
            translations,
        }
    }

    /// Look up a translation key.  Falls back to the key itself if missing.
    pub fn t<'a>(&'a self, key: &'a str) -> &'a str {
        self.translations
            .get(key)
            .map(|s| s.as_str())
            .unwrap_or(key)
    }
}
