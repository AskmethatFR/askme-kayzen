use dioxus_i18n::unic_langid::{LanguageIdentifier, langid};

pub(crate) fn detect_locale() -> LanguageIdentifier {
    select_locale(sys_locale::get_locale().as_deref())
}

pub(crate) fn select_locale(reported: Option<&str>) -> LanguageIdentifier {
    let primary = reported
        .map(|tag| tag.replace('_', "-"))
        .and_then(|normalized| normalized.split('-').next().map(str::to_string));

    match primary {
        Some(primary) if primary.eq_ignore_ascii_case("en") => langid!("en"),
        _ => langid!("fr"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_locale_maps_a_reported_tag_to_a_supported_catalogue() {
        let cases: Vec<(Option<&str>, LanguageIdentifier)> = vec![
            (Some("en-US"), langid!("en")),
            (Some("en"), langid!("en")),
            (Some("EN-us"), langid!("en")),
            (Some("en_US"), langid!("en")),
            (Some("fr-FR"), langid!("fr")),
            (Some("fr_FR"), langid!("fr")),
            (Some("de-DE"), langid!("fr")),
            (Some(""), langid!("fr")),
            (Some("???"), langid!("fr")),
            (None, langid!("fr")),
        ];

        for (reported, expected) in cases {
            assert_eq!(
                select_locale(reported),
                expected,
                "expected {reported:?} to select {expected}"
            );
        }
    }
}
