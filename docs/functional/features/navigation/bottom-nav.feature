# id: bottom-nav
# context: Navigation
# origin: issue-55
@feature:bottom-nav
Feature: Move between the three main screens from a bar at the bottom

  # First slice of the « Galets » redesign (issue #55). The bar is a new,
  # independent component; the existing in-page links stay untouched in this slice.

  @scenario:S1
  Scenario: The bar shows on the three main screens
    Given the user is on Aujourd'hui, Cette semaine or Ancrées
    Then a navigation bar sits at the bottom of the screen
    And it offers three destinations, in order: "Aujourd'hui", "Semaine", "Ancrées"

  @wip @scenario:S2
  Scenario: The current screen is marked by more than colour
    Given the user is on Cette semaine
    Then the "Semaine" destination is marked as the current page
    And that mark is announced to assistive technology, not carried by colour alone

  @scenario:S3
  Scenario: A destination opens its screen
    Given the user is on Aujourd'hui
    When the user taps "Ancrées" in the bar
    Then the Ancrées screen opens

  @scenario:S4
  Scenario: Focus screens keep the whole screen to themselves
    Given the user is on a habit's detail, in a ritual, or adding a habit
    Then no navigation bar is shown

  @scenario:S5
  Scenario: The bar speaks the user's language
    Given the app language is English
    When the user is on Aujourd'hui
    Then the three destinations read in English
