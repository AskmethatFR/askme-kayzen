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

/// The production config: the device's own locale, `fr`/`en` catalogues,
/// falling back to `fr`. Never called from a test — tests pin the locale
/// deterministically through `use_locale_for_tests`/`use_locale_for_tests_as`
/// instead of reading the real device.
pub(crate) fn config() -> I18nConfig {
    I18nConfig::new(detect_locale())
        .with_fallback(langid!("fr"))
        .with_locale((langid!("fr"), FR))
        .with_locale((langid!("en"), EN))
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

#[allow(dead_code)]
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
    dioxus_i18n::prelude::use_init_i18n(move || {
        I18nConfig::new(locale.clone())
            .with_fallback(langid!("fr"))
            .with_locale((langid!("fr"), FR))
            .with_locale((langid!("en"), EN))
    })
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
