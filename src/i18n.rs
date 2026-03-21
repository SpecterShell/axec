use std::collections::BTreeMap;
use std::env;

const FALLBACK_LOCALE: &str = "en";

pub fn init_locale() {
    rust_i18n::set_locale(&detect_locale());
}

pub fn text(key: &str) -> String {
    t!(key).to_string()
}

fn detect_locale() -> String {
    let available = rust_i18n::available_locales!();
    let raw = detect_raw_locale().unwrap_or_else(|| FALLBACK_LOCALE.to_string());
    resolve_locale(&raw, &available)
}

fn detect_raw_locale() -> Option<String> {
    ["AXEC_LOCALE", "LC_ALL", "LC_MESSAGES", "LANGUAGE", "LANG"]
        .into_iter()
        .filter_map(|name| env::var(name).ok())
        .flat_map(|value| {
            value
                .split(':')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .find(|value| !is_neutral_locale(value))
}

fn resolve_locale(raw: &str, available: &[&str]) -> String {
    let mut canonical_locales = BTreeMap::new();
    for locale in available {
        if let Some(canonical) = canonicalize_locale(locale) {
            canonical_locales.insert(canonical, *locale);
        }
    }

    if let Some(canonical) = canonicalize_locale(raw) {
        if let Some(locale) = canonical_locales.get(&canonical) {
            return (*locale).to_string();
        }

        let parts = canonical.split('-').collect::<Vec<_>>();
        if parts.first().copied() == Some("zh") {
            let preferred = if looks_traditional_chinese(&parts) {
                "zh-TW"
            } else {
                "zh-CN"
            };
            if let Some(locale) = canonical_locales.get(preferred) {
                return (*locale).to_string();
            }
        }
    }

    for candidate in locale_candidates(raw) {
        if let Some(locale) = canonical_locales.get(&candidate) {
            return (*locale).to_string();
        }
    }

    if canonical_locales.contains_key(FALLBACK_LOCALE) {
        return FALLBACK_LOCALE.to_string();
    }

    available
        .first()
        .copied()
        .unwrap_or(FALLBACK_LOCALE)
        .to_string()
}

fn locale_candidates(raw: &str) -> Vec<String> {
    let Some(canonical) = canonicalize_locale(raw) else {
        return vec![FALLBACK_LOCALE.to_string()];
    };

    let parts = canonical.split('-').collect::<Vec<_>>();
    let mut candidates = Vec::new();

    push_candidate(&mut candidates, canonical.clone());

    if parts.first().copied() == Some("zh") {
        if looks_traditional_chinese(&parts) {
            push_candidate(&mut candidates, "zh-TW".to_string());
        } else {
            push_candidate(&mut candidates, "zh-CN".to_string());
        }
    }

    for end in (1..parts.len()).rev() {
        push_candidate(&mut candidates, parts[..end].join("-"));
    }

    if let Some(language) = parts.first() {
        push_candidate(&mut candidates, (*language).to_string());
    }

    push_candidate(&mut candidates, FALLBACK_LOCALE.to_string());
    candidates
}

fn push_candidate(candidates: &mut Vec<String>, candidate: String) {
    if !candidates.iter().any(|existing| existing == &candidate) {
        candidates.push(candidate);
    }
}

fn canonicalize_locale(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let core = trimmed
        .split('.')
        .next()
        .unwrap_or(trimmed)
        .split('@')
        .next()
        .unwrap_or(trimmed)
        .replace('_', "-");

    let parts = core
        .split('-')
        .filter(|part| !part.is_empty())
        .enumerate()
        .map(|(index, part)| normalize_locale_part(index, part))
        .collect::<Vec<_>>();

    if parts.is_empty() {
        None
    } else {
        Some(parts.join("-"))
    }
}

fn normalize_locale_part(index: usize, part: &str) -> String {
    if part.is_empty() {
        return String::new();
    }

    if index == 0 {
        return part.to_ascii_lowercase();
    }

    if part.len() == 4 && part.chars().all(|ch| ch.is_ascii_alphabetic()) {
        let mut chars = part.chars();
        let mut normalized = String::new();
        if let Some(first) = chars.next() {
            normalized.push(first.to_ascii_uppercase());
        }
        normalized.extend(chars.map(|ch| ch.to_ascii_lowercase()));
        return normalized;
    }

    if (part.len() == 2 && part.chars().all(|ch| ch.is_ascii_alphabetic()))
        || (part.len() == 3 && part.chars().all(|ch| ch.is_ascii_digit()))
    {
        return part.to_ascii_uppercase();
    }

    part.to_ascii_lowercase()
}

fn looks_traditional_chinese(parts: &[&str]) -> bool {
    parts.iter().skip(1).any(|part| {
        matches!(
            *part,
            "TW" | "HK" | "MO" | "Hant" | "hant" | "Macau" | "macau"
        )
    })
}

fn is_neutral_locale(value: &str) -> bool {
    matches!(value, "C" | "POSIX") || value.starts_with("C.")
}

#[cfg(test)]
mod tests {
    use super::resolve_locale;

    #[test]
    fn resolves_exact_and_base_locales() {
        let available = ["en", "fr", "zh-CN", "zh-TW"];
        assert_eq!(resolve_locale("en_US.UTF-8", &available), "en");
        assert_eq!(resolve_locale("fr_CA.UTF-8", &available), "fr");
    }

    #[test]
    fn resolves_traditional_chinese_aliases() {
        let available = ["en", "zh-CN", "zh-TW"];
        assert_eq!(resolve_locale("zh_TW.UTF-8", &available), "zh-TW");
        assert_eq!(resolve_locale("zh-Hant-HK", &available), "zh-TW");
        assert_eq!(resolve_locale("zh_HK.UTF-8", &available), "zh-TW");
    }

    #[test]
    fn resolves_simplified_chinese_aliases() {
        let available = ["en", "zh-CN", "zh-TW"];
        assert_eq!(resolve_locale("zh_CN.UTF-8", &available), "zh-CN");
        assert_eq!(resolve_locale("zh-Hans-SG", &available), "zh-CN");
    }

    #[test]
    fn falls_back_to_english_when_locale_is_missing() {
        let available = ["en", "zh-CN", "zh-TW"];
        assert_eq!(resolve_locale("de_DE.UTF-8", &available), "en");
    }

    #[test]
    fn prefers_new_locales_when_they_exist() {
        let available = ["en", "ja", "zh-CN", "zh-TW"];
        assert_eq!(resolve_locale("ja_JP.UTF-8", &available), "ja");
    }
}
