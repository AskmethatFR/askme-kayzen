pub const DARK_GROUND_JS: &str =
    "if (window.AskmeSystemBars) { window.AskmeSystemBars.darkGround(true); }";
pub const PAPER_GROUND_JS: &str =
    "if (window.AskmeSystemBars) { window.AskmeSystemBars.darkGround(false); }";

#[cfg(test)]
mod tests {
    use super::*;

    const SEAM_SOURCE: &str = include_str!("../../scripts/android-release-lib.sh");

    fn seam_block() -> &'static str {
        let opener = "block=\"$(cat <<'KOTLIN'\n";
        let start = SEAM_SOURCE
            .find(opener)
            .expect("the release seam carries the MainActivity block heredoc")
            + opener.len();
        let end = SEAM_SOURCE[start..]
            .find("\nKOTLIN\n")
            .expect("the MainActivity block heredoc closes on KOTLIN");
        &SEAM_SOURCE[start..start + end]
    }

    fn position(haystack: &str, needle: &str) -> usize {
        haystack
            .find(needle)
            .unwrap_or_else(|| panic!("expected {needle:?} in:\n{haystack}"))
    }

    // @scenario: safe-area-bleed/S4
    #[test]
    fn safe_area_bleed_s4_given_the_release_seam_then_one_boolean_switch_turns_both_bars_on_the_ui_thread()
     {
        let block = seam_block();

        let hook = position(block, "override fun onWebViewCreate(");
        let register = position(
            block,
            r#"webView.addJavascriptInterface(AskmeSystemBars(this), "AskmeSystemBars")"#,
        );
        assert!(
            hook < register,
            "the switch is registered inside onWebViewCreate, before the page loads"
        );
        assert_eq!(
            block.matches("\"AskmeSystemBars\"").count(),
            1,
            "the switch is registered under the AskmeSystemBars name exactly once"
        );

        let bridge = &block
            [position(block, "class AskmeSystemBars(")..position(block, "class AskmeSafeArea {")];
        assert_eq!(
            bridge
                .matches("@android.webkit.JavascriptInterface")
                .count(),
            1,
            "the switch exposes exactly one member to page scripts"
        );
        let member = position(
            bridge,
            "@android.webkit.JavascriptInterface\n    fun darkGround(dark: Boolean) {",
        );
        let ui_thread = position(bridge, "activity.runOnUiThread {");
        let status = position(bridge, "isAppearanceLightStatusBars = !dark");
        let navigation = position(bridge, "isAppearanceLightNavigationBars = !dark");
        assert!(
            member < ui_thread && ui_thread < status && ui_thread < navigation,
            "darkGround takes one Boolean, returns nothing, and switches both bars on the UI \
             thread (page-script calls arrive on the JavaBridge thread)"
        );
        assert!(
            !bridge.contains("String") && !bridge.contains("WebView"),
            "the switch reads no string and holds no WebView: one boolean in, nothing out"
        );
    }

    // @scenario: safe-area-bleed/S4
    #[test]
    fn safe_area_bleed_s4_given_the_page_scripts_then_each_flips_the_switch_behind_the_absent_bridge_guard()
     {
        for (script, dark) in [(DARK_GROUND_JS, "true"), (PAPER_GROUND_JS, "false")] {
            let guard = position(script, "if (window.AskmeSystemBars) {");
            let call = position(
                script,
                &format!("window.AskmeSystemBars.darkGround({dark});"),
            );
            assert!(
                guard < call,
                "the switch is flipped only when the bridge exists (web, iOS, debug: no-op)"
            );
            assert_eq!(
                script.matches("darkGround(").count(),
                1,
                "each script is one fixed call with a literal boolean, never built from data"
            );
        }
    }
}
