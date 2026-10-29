use std::{collections::BTreeMap, sync::LazyLock};

#[derive(Clone, Copy)]
pub struct Language {
    pub code: &'static str,
    pub native: &'static str,
}
pub const LANGUAGES: [Language; 20] = [
    Language {
        code: "en",
        native: "English",
    },
    Language {
        code: "tr",
        native: "Türkçe",
    },
    Language {
        code: "az",
        native: "Azərbaycanca",
    },
    Language {
        code: "hi",
        native: "हिन्दी",
    },
    Language {
        code: "ru",
        native: "Русский",
    },
    Language {
        code: "es",
        native: "Español",
    },
    Language {
        code: "pt",
        native: "Português",
    },
    Language {
        code: "fr",
        native: "Français",
    },
    Language {
        code: "de",
        native: "Deutsch",
    },
    Language {
        code: "ar",
        native: "العربية",
    },
    Language {
        code: "zh",
        native: "简体中文",
    },
    Language {
        code: "ja",
        native: "日本語",
    },
    Language {
        code: "ko",
        native: "한국어",
    },
    Language {
        code: "it",
        native: "Italiano",
    },
    Language {
        code: "nl",
        native: "Nederlands",
    },
    Language {
        code: "pl",
        native: "Polski",
    },
    Language {
        code: "uk",
        native: "Українська",
    },
    Language {
        code: "id",
        native: "Bahasa Indonesia",
    },
    Language {
        code: "vi",
        native: "Tiếng Việt",
    },
    Language {
        code: "th",
        native: "ไทย",
    },
];
static STRINGS: LazyLock<BTreeMap<String, Vec<String>>> = LazyLock::new(|| {
    let mut all = BTreeMap::new();
    for source in [
        include_str!("../locales/core.json"),
        include_str!("../locales/preferences.json"),
        include_str!("../locales/feedback.json"),
    ] {
        let part: BTreeMap<String, Vec<String>> =
            serde_json::from_str(source).expect("valid bundled translations");
        for (key, values) in part {
            assert!(
                all.insert(key, values).is_none(),
                "duplicate translation key"
            );
        }
    }
    all
});
pub fn text(locale: &str, key: &str) -> String {
    let index = LANGUAGES.iter().position(|l| l.code == locale).unwrap_or(0);
    let value = STRINGS
        .get(key)
        .and_then(|row| row.get(index).or_else(|| row.first()))
        .map(String::as_str)
        .unwrap_or(key);
    visual_text(value, locale == "ar")
}
pub fn visual_text(text: &str, rtl: bool) -> String {
    if !rtl {
        return text.into();
    }
    // Reorder bidi runs visually, but keep Arabic letter runs in logical order:
    // epaint 0.37's HarfRust shaper already reverses/shapes those font runs.
    // Pre-shaped presentation forms would be reversed twice.
    let bidi = unicode_bidi::BidiInfo::new(text, Some(unicode_bidi::Level::rtl()));
    bidi.paragraphs
        .iter()
        .map(|p| {
            let visual = bidi.reorder_line(p, p.range.clone());
            let mut result = String::new();
            let mut arabic = String::new();
            let flush = |result: &mut String, run: &mut String| {
                result.extend(run.chars().rev());
                run.clear();
            };
            for ch in visual.chars() {
                if matches!(
                    unicode_bidi::bidi_class(ch),
                    unicode_bidi::BidiClass::AL | unicode_bidi::BidiClass::NSM
                ) {
                    arabic.push(ch);
                } else {
                    flush(&mut result, &mut arabic);
                    result.push(ch);
                }
            }
            flush(&mut result, &mut arabic);
            result
        })
        .collect::<Vec<_>>()
        .join("\n")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_label_has_twenty_nonempty_translations() {
        assert!(STRINGS.len() >= 90);
        for (key, row) in STRINGS.iter() {
            assert_eq!(row.len(), 20, "{key}");
            assert!(row.iter().all(|s| !s.trim().is_empty()), "{key}");
        }
        for binding in crate::shortcuts::Shortcuts::default().bindings {
            assert!(
                STRINGS.contains_key(binding.action.label()),
                "{}",
                binding.action.label()
            );
        }
    }
    #[test]
    fn unknown_languages_fall_back_to_english() {
        assert_eq!(text("unknown", "add"), text("en", "add"));
    }
    #[test]
    fn arabic_visual_order_preserves_logical_words_and_latin_version_numbers() {
        let text = visual_text("الإصدار 0.2.0", true);
        assert!(text.contains("0.2.0"));
        assert_ne!(text, "الإصدار 0.2.0");
        assert!(text.contains("الإصدار"));
        assert_eq!(visual_text("مساحة العمل", true), "العمل مساحة");
    }
}
