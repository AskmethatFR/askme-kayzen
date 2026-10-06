pub const BOOTSTRAP_JS: &str = r#"
window.__kzApplySafeAreaInsets = function (top, right, bottom, left) {
  var style = document.documentElement.style;
  style.setProperty("--safe-area-inset-top", top + "px");
  style.setProperty("--safe-area-inset-right", right + "px");
  style.setProperty("--safe-area-inset-bottom", bottom + "px");
  style.setProperty("--safe-area-inset-left", left + "px");
};
var bridge = window.AskmeSafeArea;
if (bridge) {
  var insets = bridge.insets().split(" ");
  window.__kzApplySafeAreaInsets(
    parseFloat(insets[0]),
    parseFloat(insets[1]),
    parseFloat(insets[2]),
    parseFloat(insets[3])
  );
}
"#;

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

    // @scenario: safe-area-bleed/S2
    #[test]
    fn safe_area_bleed_s2_given_the_release_seam_then_it_registers_one_read_only_bridge_under_on_web_view_create()
     {
        let block = seam_block();

        assert_eq!(
            block.matches("\"AskmeSafeArea\"").count(),
            1,
            "the bridge must be registered under the AskmeSafeArea name exactly once"
        );
        assert_eq!(
            block.matches("addJavascriptInterface(").count(),
            1,
            "exactly one addJavascriptInterface call in the seam block"
        );
        let hook = block
            .find("override fun onWebViewCreate(")
            .expect("the seam overrides WryActivity.onWebViewCreate");
        let register = block
            .find("addJavascriptInterface(")
            .expect("the seam registers the bridge");
        assert!(
            register > hook,
            "the bridge must be registered inside onWebViewCreate, before the page loads"
        );

        assert_eq!(
            block.matches("@android.webkit.JavascriptInterface").count(),
            1,
            "exactly one @JavascriptInterface member: insets(), geometry only"
        );
        assert!(
            block.contains("fun insets(): String"),
            "the bridge exposes the 0-arg insets(): String reader"
        );
        let annotation = block
            .find("@android.webkit.JavascriptInterface")
            .expect("the bridge carries the @JavascriptInterface annotation");
        let reader = block
            .find("fun insets(): String")
            .expect("the bridge carries the insets() reader");
        assert!(
            annotation < reader,
            "the @JavascriptInterface annotation must sit on insets(), not elsewhere"
        );

        assert_eq!(
            block.matches("@Volatile").count(),
            1,
            "the bridge caches its snapshot in one @Volatile field"
        );
        assert_eq!(
            block.matches("%.2f").count(),
            4,
            "the bridge formats all four insets as Locale.US %.2f CSS px"
        );
        assert!(
            block.contains("java.util.Locale.US"),
            "the bridge formats numbers under Locale.US so the JS parse is locale-proof"
        );
        assert!(
            block.contains("view.resources.displayMetrics.density"),
            "the bridge converts physical px to CSS px through the display density"
        );

        assert_eq!(
            block.matches("window.__kzApplySafeAreaInsets(").count(),
            1,
            "the seam pushes through the single writer entry point"
        );
        assert_eq!(
            block.matches("evaluateJavascript(").count(),
            1,
            "the seam evaluates exactly one push script"
        );
        assert!(
            block.contains("webView.post {"),
            "the push is posted to the WebView thread like wry's own evalScript"
        );

        for needle in [
            "WindowInsetsCompat.Type.systemBars()",
            "WindowInsetsCompat.Type.displayCutout()",
            "WindowInsetsCompat.Type.ime()",
        ] {
            assert!(
                block.contains(needle),
                "the insets listener must read {needle} so rotation and keyboard both refresh"
            );
        }

        let bridge = &block[block
            .find("class AskmeSafeArea {")
            .expect("the seam declares the AskmeSafeArea bridge class")..];
        assert!(
            !bridge.contains("WebView") && !bridge.contains("Activity"),
            "the bridge class must hold no Activity/WebView reference — one cached string only"
        );
    }

    // @scenario: safe-area-bleed/S2
    #[test]
    fn safe_area_bleed_s2_given_the_bootstrap_script_then_it_pulls_once_behind_the_absent_bridge_guard()
     {
        assert_eq!(
            BOOTSTRAP_JS
                .matches("window.__kzApplySafeAreaInsets = ")
                .count(),
            1,
            "the script defines the writer window.__kzApplySafeAreaInsets exactly once"
        );
        for side in ["top", "right", "bottom", "left"] {
            assert_eq!(
                BOOTSTRAP_JS
                    .matches(&format!("\"--safe-area-inset-{side}\""))
                    .count(),
                1,
                "the writer sets --safe-area-inset-{side} exactly once"
            );
        }

        assert_eq!(
            BOOTSTRAP_JS.matches("AskmeSafeArea").count(),
            1,
            "the script names the bridge exactly once"
        );
        assert_eq!(
            BOOTSTRAP_JS.matches(".insets()").count(),
            1,
            "the script pulls the bridge snapshot exactly once"
        );
        assert!(
            BOOTSTRAP_JS.contains("var bridge = window.AskmeSafeArea;"),
            "the script captures the bridge once, before the guard"
        );
        let guard = BOOTSTRAP_JS
            .find("if (bridge)")
            .expect("the script guards the pull behind the bridge's presence");
        let pull = BOOTSTRAP_JS
            .find(".insets()")
            .expect("the script pulls the bridge snapshot");
        let call = BOOTSTRAP_JS
            .find("window.__kzApplySafeAreaInsets(")
            .expect("the script invokes the writer once, from the pull");
        let definition = BOOTSTRAP_JS
            .find("window.__kzApplySafeAreaInsets = ")
            .expect("the script defines the writer");
        assert!(
            definition < guard,
            "the writer must exist before the guard so an insets-change push can land"
        );
        assert!(
            guard < pull && pull < call,
            "the pull and the writer call must sit inside the absent-bridge guard: vars stay \
             unset when AskmeSafeArea is missing, so the env() arm keeps deciding"
        );
    }

    // @scenario: safe-area-bleed/S1
    #[test]
    fn safe_area_bleed_s1_given_the_seam_and_stylesheet_then_the_page_paints_under_the_system_bars()
    {
        const MAIN_CSS: &str = include_str!("../assets/main.css");
        let block = seam_block();

        assert_eq!(
            block.matches("view.setPadding(").count(),
            0,
            "decor-view padding is what kept the inset bands out of the page's reach (AC1)"
        );
        assert!(
            block.contains("WindowCompat.setDecorFitsSystemWindows(window, false)"),
            "the seam keeps edge-to-edge enabled so the page spans the screen"
        );
        assert!(
            MAIN_CSS.contains("html, body {\n    margin: 0;\n    padding: 0;\n}"),
            "the root box must carry no margin/padding of its own"
        );
        assert!(
            MAIN_CSS.contains("body {\n    background: var(--color-paper);"),
            "the page background must paint paper to the screen edges"
        );
    }
}
