use crate::route::{Route, back_target};
use dioxus::prelude::*;

pub const BACK_JS: &str = r#"
var canGoUp = false;
if (!Object.prototype.hasOwnProperty.call(window, "__kzBack")) {
  Object.defineProperty(window, "__kzBack", {
    value: function () {
      if (canGoUp !== true) {
        return false;
      }
      dioxus.send(true);
      return true;
    },
    writable: false,
    configurable: false
  });
}
try {
  while (true) {
    canGoUp = await dioxus.recv();
  }
} finally {
  canGoUp = false;
}
"#;

#[component]
pub fn BackKey() -> Element {
    let bridge = use_hook(|| document::eval(BACK_JS));
    // @algo: told during render, before any queued Back press is served; a repeat is harmless.
    let _ = bridge.send(back_target(&use_route::<Route>()).is_some());
    use_future(move || async move {
        let mut bridge = bridge;
        while bridge.recv::<bool>().await.is_ok() {
            if let Some(up) = back_target(&router().current::<Route>()) {
                navigator().push(up);
            }
        }
    });
    rsx! {
        Outlet::<Route> {}
    }
}

#[cfg(test)]
mod tests {
    const SEAM_SOURCE: &str = include_str!("../../scripts/android-release-lib.sh");
    const ASK_THE_PAGE: &str = r#"webView.evaluateJavascript("window.__kzBack()") { answer ->"#;

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

    // @scenario: system-back/S2
    #[test]
    fn system_back_s2_given_the_page_script_then_it_is_exactly_the_fail_closed_text() {
        assert_eq!(
            super::BACK_JS,
            r#"
var canGoUp = false;
if (!Object.prototype.hasOwnProperty.call(window, "__kzBack")) {
  Object.defineProperty(window, "__kzBack", {
    value: function () {
      if (canGoUp !== true) {
        return false;
      }
      dioxus.send(true);
      return true;
    },
    writable: false,
    configurable: false
  });
}
try {
  while (true) {
    canGoUp = await dioxus.recv();
  }
} finally {
  canGoUp = false;
}
"#,
            "the page side of Back is one pinned text: a non-writable, non-configurable \
             __kzBack, only a strict true opens it, and a dead channel closes it again"
        );
    }

    // @scenario: system-back/S1
    #[test]
    fn system_back_s1_given_the_release_seam_then_back_asks_the_page_one_fixed_question() {
        let block = seam_block();

        let hook = position(block, "override fun onWebViewCreate(");
        let register = position(
            block,
            "onBackPressedDispatcher.addCallback(this, object : androidx.activity.OnBackPressedCallback(true) {",
        );
        let handler = position(block, "override fun handleOnBackPressed() {");
        let ask = position(block, ASK_THE_PAGE);
        assert!(
            hook < register && register < handler && handler < ask,
            "the Back callback is registered once the WebView exists and asks the page from \
             handleOnBackPressed"
        );
        assert_eq!(block.matches("addCallback(").count(), 1);
        assert_eq!(
            block.matches("__kzBack").count(),
            1,
            "the Back question is the one fixed script, never built from data"
        );
    }

    // @scenario: system-back/S2
    #[test]
    fn system_back_s2_given_the_release_seam_then_any_answer_but_true_hands_back_to_the_system() {
        let block = seam_block();
        let asked = &block[position(block, ASK_THE_PAGE)..];
        let answer_lambda = &asked[..position(asked, "\n            }\n")];
        assert_eq!(
            answer_lambda,
            concat!(
                "webView.evaluateJavascript(\"window.__kzBack()\") { answer ->\n",
                "                    if (answer != \"true\") {\n",
                "                        isEnabled = false\n",
                "                        onBackPressedDispatcher.onBackPressed()\n",
                "                        isEnabled = true\n",
                "                    }\n",
                "                }"
            ),
            "the answer lambda is exactly the fail-closed lines: nothing runs before the \
             answer is compared to true, nothing swallows Back on another answer"
        );

        let fail_closed = position(asked, "if (answer != \"true\") {");
        let disable = position(asked, "isEnabled = false");
        let redispatch = position(asked, "onBackPressedDispatcher.onBackPressed()");
        let enable = position(asked, "isEnabled = true");
        assert!(
            fail_closed < disable && disable < redispatch && redispatch < enable,
            "anything but the JSON answer true disables the callback, re-dispatches Back to the \
             system, then re-arms the callback for when the user comes back"
        );
    }
}
