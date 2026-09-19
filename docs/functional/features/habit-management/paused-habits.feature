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
