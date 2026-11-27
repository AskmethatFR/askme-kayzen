# id: paused-habits
# context: HabitManagement
# origin: issue-59
@feature:paused-habits
Feature: The habits at rest live on their own screen

  # Issue #59 — the paused zone leaves Aujourd'hui for a dedicated screen,
  # because the owner can hold many habits in pause at once.

  @scenario:S1
  Scenario: The paused screen lists the habits at rest
    Given a board holding two habits in pause
    When the user opens the paused screen
    Then it lists both habits
    And each one offers « La reprendre »

  @scenario:S2
  Scenario: Resuming from the paused screen brings the habit back
    Given a paused habit listed on the paused screen
    When the user resumes it
    Then it is active again and leaves the paused screen
    And its completion history is untouched

  @scenario:S3
  Scenario: A tap anywhere on a paused habit's card opens its detail, « La reprendre » keeping its own tap
    Given a board holding two habits in pause
    When the paused screen renders
    Then each habit's title link covers its whole card with its invisible layer
    And « La reprendre » stays above that layer, so a tap on it resumes the habit and does not open the detail
