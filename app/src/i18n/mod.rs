mod locale_choice;
#[cfg(test)]
mod route_smoke;

use dioxus_i18n::prelude::I18nConfig;
use dioxus_i18n::unic_langid::langid;

pub(crate) use locale_choice::detect_locale;
#[cfg(test)]
pub(crate) use locale_choice::select_locale;

const FR: &str = include_str!("fr.ftl");
const EN: &str = include_str!("en.ftl");

/// `FluentBundle::new` (called by `dioxus-i18n` 0.5.1) never disables
/// `use_isolating`, so fluent-rs always wraps an interpolated placeable in
/// FSI/PDI marks (U+2066..U+2069) — correct for a catalogue mixing RTL and
/// LTR text, noise for an app shipping `fr`/`en` only. Stripped here, once,
/// so every call site through `tr!`/`tr_key` gets clean text.
pub(crate) fn strip_isolates(text: String) -> String {
    text.chars()
        .filter(|c| !matches!(c, '\u{2066}'..='\u{2069}'))
        .collect()
}

/// The one place both catalogues are registered — the production `config()`
/// and the test seam `use_locale_for_tests_as` both build on this, so a
/// dropped `.with_locale` here breaks every English-locale test in the
/// suite, not just production. `.with_fallback(fr)` is currently
/// unreachable — `select_locale`'s range is only `fr`/`en`, both already
/// registered via `.with_locale` — but it stays as the config-boundary
/// safety net for the day `select_locale` carries a wider range (e.g. an
/// in-app language switcher). The fallback *decision* for an unsupported
/// device locale lives in `select_locale`, not here.
fn config_for(locale: dioxus_i18n::unic_langid::LanguageIdentifier) -> I18nConfig {
    I18nConfig::new(locale)
        .with_fallback(langid!("fr"))
        .with_locale((langid!("fr"), FR))
        .with_locale((langid!("en"), EN))
}

/// The production config: the device's own locale, `fr`/`en` catalogues,
/// falling back to `fr`. Never called from a test — tests pin the locale
/// deterministically through `use_locale_for_tests`/`use_locale_for_tests_as`
/// instead of reading the real device.
pub(crate) fn config() -> I18nConfig {
    config_for(detect_locale())
}

macro_rules! tr {
    ($id:expr $(,)?) => {
        $crate::i18n::strip_isolates(::dioxus_i18n::t!($id))
    };
    ($id:expr, $( $name:ident : $value:expr ),+ $(,)?) => {
        $crate::i18n::strip_isolates(::dioxus_i18n::t!($id, $( $name : $value ),+))
    };
}
pub(crate) use tr;

pub(crate) fn tr_key(key: &str) -> String {
    strip_isolates(::dioxus_i18n::prelude::i18n().translate(key))
}

/// Test seam: pins the i18n context to a known locale without ever reading
/// the platform, so a test's outcome depends only on the locale it asked
/// for. Every existing French assertion in this suite relies on `fr`, hence
/// the no-argument default.
#[cfg(test)]
pub(crate) fn use_locale_for_tests() -> dioxus_i18n::prelude::I18n {
    use_locale_for_tests_as(langid!("fr"))
}

#[cfg(test)]
pub(crate) fn use_locale_for_tests_as(
    locale: dioxus_i18n::unic_langid::LanguageIdentifier,
) -> dioxus_i18n::prelude::I18n {
    dioxus_i18n::prelude::use_init_i18n(move || config_for(locale.clone()))
}

/// Test seam: every message id each catalogue actually defines, parsed
/// statically (no rendering, no randomness) — the seam a view's own test
/// module uses to prove a catalogue key it returns (an idea key, a recap/week
/// copy key, a refusal key) resolves in both languages, instead of only
/// discovering a typo the one render in ten thousand that happens to pick it.
#[cfg(test)]
pub(crate) fn catalogue_ids() -> (
    std::collections::BTreeSet<String>,
    std::collections::BTreeSet<String>,
) {
    (message_ids(FR, "fr.ftl"), message_ids(EN, "en.ftl"))
}

#[cfg(test)]
fn message_ids(text: &'static str, file_name: &str) -> std::collections::BTreeSet<String> {
    fluent_syntax::parser::parse(text)
        .unwrap_or_else(|(_, errors)| panic!("{file_name} failed to parse: {errors:?}"))
        .body
        .into_iter()
        .map(|entry| match entry {
            fluent_syntax::ast::Entry::Message(message) => message.id.name.to_string(),
            other => panic!("{file_name} contains a non-message entry: {other:?}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};

    #[test]
    fn strip_isolates_removes_every_bidi_isolate_mark() {
        let cases: Vec<(&str, &str)> = vec![
            ("no marks here", "no marks here"),
            ("\u{2066}1\u{2069} sur \u{2066}1\u{2069}", "1 sur 1"),
            ("\u{2067}right-to-left\u{2069}", "right-to-left"),
            ("\u{2068}first-strong\u{2069}", "first-strong"),
            ("", ""),
        ];

        for (input, expected) in cases {
            assert_eq!(
                strip_isolates(input.to_string()),
                expected,
                "expected isolate marks stripped from {input:?}"
            );
        }
    }

    #[test]
    fn fr_and_en_catalogues_expose_the_same_message_ids_and_the_same_variables_per_message() {
        let fr = parse_catalogue(FR, "fr.ftl");
        let en = parse_catalogue(EN, "en.ftl");

        let fr_ids: BTreeSet<&String> = fr.keys().collect();
        let en_ids: BTreeSet<&String> = en.keys().collect();
        assert_eq!(
            fr_ids, en_ids,
            "expected fr.ftl and en.ftl to define the same message ids"
        );

        for (id, fr_variables) in &fr {
            let en_variables = &en[id];
            assert_eq!(
                fr_variables, en_variables,
                "expected message '{id}' to reference the same variables in fr.ftl and en.ftl"
            );
        }
    }

    #[test]
    fn no_english_message_is_byte_identical_to_its_french_counterpart() {
        let fr = message_texts(FR, "fr.ftl");
        let en = message_texts(EN, "en.ftl");

        let untranslated: Vec<&String> = fr
            .keys()
            .filter(|id| en.get(*id).is_some_and(|value| value == &fr[*id]))
            .collect();

        assert!(
            untranslated.is_empty(),
            "expected every English message to differ from its French counterpart \
             (a partial catalogue must fail the build, never render mixed languages), \
             found identical text for: {untranslated:?}"
        );
    }

    #[test]
    fn every_aria_reads_its_label_followed_by_the_title_placeholder() {
        for (file_name, text) in [("fr.ftl", FR), ("en.ftl", EN)] {
            let messages = message_texts(text, file_name);

            for label_id in messages.keys().filter(|id| id.ends_with("-label")) {
                let aria_id = format!("{}-aria", label_id.strip_suffix("-label").unwrap());
                let Some(aria_serialized) = messages.get(&aria_id) else {
                    continue;
                };

                let label_value = message_value(label_id, &messages[label_id]);
                let aria_value = message_value(&aria_id, aria_serialized);
                let expected = format!("{label_value} · {{ $title }}");

                assert_eq!(
                    aria_value, expected,
                    "expected {aria_id} to read as {label_id} followed by ' · {{ $title }}' \
                     in {file_name}, got: {aria_value:?}"
                );
            }
        }
    }

    fn message_value(id: &str, serialized: &str) -> String {
        serialized
            .strip_prefix(&format!("{id} = "))
            .unwrap_or(serialized)
            .trim_end_matches('\n')
            .to_string()
    }

    fn message_texts(text: &'static str, file_name: &str) -> BTreeMap<String, String> {
        let resource = fluent_syntax::parser::parse(text)
            .unwrap_or_else(|(_, errors)| panic!("{file_name} failed to parse: {errors:?}"));

        resource
            .body
            .into_iter()
            .map(|entry| match entry {
                fluent_syntax::ast::Entry::Message(message) => {
                    let id = message.id.name.to_string();
                    let wrapped = fluent_syntax::ast::Resource {
                        body: vec![fluent_syntax::ast::Entry::Message(message)],
                    };
                    (id, fluent_syntax::serializer::serialize(&wrapped))
                }
                other => panic!("{file_name} contains a non-message entry: {other:?}"),
            })
            .collect()
    }

    fn parse_catalogue(text: &'static str, file_name: &str) -> BTreeMap<String, BTreeSet<String>> {
        let resource = fluent_syntax::parser::parse(text)
            .unwrap_or_else(|(_, errors)| panic!("{file_name} failed to parse: {errors:?}"));

        resource
            .body
            .into_iter()
            .map(|entry| match entry {
                fluent_syntax::ast::Entry::Message(message) => {
                    let variables = message.value.map(pattern_variables).unwrap_or_default();
                    (message.id.name.to_string(), variables)
                }
                other => panic!("{file_name} contains a non-message entry: {other:?}"),
            })
            .collect()
    }

    fn pattern_variables(pattern: fluent_syntax::ast::Pattern<&str>) -> BTreeSet<String> {
        let mut variables = BTreeSet::new();
        for element in &pattern.elements {
            collect_pattern_element_variables(element, &mut variables);
        }
        variables
    }

    fn collect_pattern_element_variables(
        element: &fluent_syntax::ast::PatternElement<&str>,
        out: &mut BTreeSet<String>,
    ) {
        if let fluent_syntax::ast::PatternElement::Placeable { expression } = element {
            collect_expression_variables(expression, out);
        }
    }

    fn collect_expression_variables(
        expression: &fluent_syntax::ast::Expression<&str>,
        out: &mut BTreeSet<String>,
    ) {
        match expression {
            fluent_syntax::ast::Expression::Inline(inline) => collect_inline_variables(inline, out),
            fluent_syntax::ast::Expression::Select { selector, variants } => {
                collect_inline_variables(selector, out);
                for variant in variants {
                    for element in &variant.value.elements {
                        collect_pattern_element_variables(element, out);
                    }
                }
            }
        }
    }

    fn collect_inline_variables(
        inline: &fluent_syntax::ast::InlineExpression<&str>,
        out: &mut BTreeSet<String>,
    ) {
        match inline {
            fluent_syntax::ast::InlineExpression::VariableReference { id } => {
                out.insert(id.name.to_string());
            }
            fluent_syntax::ast::InlineExpression::FunctionReference { arguments, .. } => {
                for positional in &arguments.positional {
                    collect_inline_variables(positional, out);
                }
                for named in &arguments.named {
                    collect_inline_variables(&named.value, out);
                }
            }
            fluent_syntax::ast::InlineExpression::Placeable { expression } => {
                collect_expression_variables(expression.as_ref(), out);
            }
            _ => {}
        }
    }
}
