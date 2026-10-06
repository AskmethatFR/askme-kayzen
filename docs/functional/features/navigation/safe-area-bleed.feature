# id: safe-area-bleed
# context: Navigation
# origin: issue-79
@feature:safe-area-bleed
Feature: Content paints under the system bars and respects the device insets

  The app draws edge-to-edge: the page background reaches the screen edges and
  the web layer keeps content clear of the device insets (camera cutout, system
  bars). The insets reach the web layer from the native side on Android
  (env(safe-area-inset-*) measures 0 in the Android WebView — adr-0021); the
  env() form survives as the web/iOS fallback arm on every site.

  @scenario:S1
  Scenario: The page paints under the camera cutout and the nav bar
    Given the app on a device with a camera cutout and a bottom system bar
    Then the page background reaches the top and bottom screen edges
    And no system-bar band of window background is visible above or below it

  @scenario:S2
  Scenario: Content sits inside the reported insets
    Given the app on a device with a 120px camera cutout and a 142px nav bar
    Then the greeting header is fully visible below the cutout
    And the bottom navigation pill is fully tappable above the nav bar

  @scenario:S3
  Scenario: The web/iOS fallback arm survives on every safe-area site
    Given the stylesheet of the app
    Then every safe-area site reads var(--safe-area-inset-*, env(safe-area-inset-*, 0px))
    And the source contract pins exactly 15 safe-area sites
