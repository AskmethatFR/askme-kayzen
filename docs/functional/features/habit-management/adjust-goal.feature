# id: adjust-goal
# context: HabitManagement
# origin: slice-3
@feature:adjust-goal
Feature: Adjust a habit's goal at the user's own pace

  # The goal only ever moves through these gestures — the system never detects
  # stability and never suggests a change (adr-0008-goal-based-dose-user-paced-progression).

  @scenario:S1
  Scenario: Growing a habit raises its goal by one minute
    Given a habit with a goal of 5 minutes
    When the user chooses "grandir"
    Then the goal becomes 6 minutes
    And the change is added to the habit's record with today's date

  @scenario:S2
  Scenario: Lightening a habit lowers its goal by one minute
    Given a habit with a goal of 5 minutes
    When the user chooses "alléger"
    Then the goal becomes 4 minutes
    And the change is added to the habit's record with today's date

  @scenario:S3
  Scenario: Lightening a habit already at one minute changes nothing
    Given a habit with a goal of 1 minute
    When the user chooses "alléger"
    Then the goal stays at 1 minute, because one minute is the floor

  @scenario:S4
  Scenario: Both gestures stay available whatever the habit's history
    Given a habit whatever its completions and its current goal
    When the user opens its detail
    Then both "grandir" and "alléger" are offered, because progression is user-paced

  @scenario:S5
  Scenario: Growing a habit already at its maximum changes nothing
    Given a habit with a goal at its maximum
    When the user chooses "grandir"
    Then the goal stays at its maximum, and nothing is added to its record

  @scenario:S6
  Scenario: Button labels show the reachable next goal at a normal effort level
    Given a habit with a goal of 5 minutes
    When the user opens its detail
    Then the "Grandir" button shows "Passer à 6 min"
    And the "Alléger" button shows "Alléger à 4 min"

  @scenario:S7
  Scenario: At the floor, the "Alléger" button shows the floor itself, not an error
    Given a habit with a goal of 1 minute
    When the user opens its detail
    Then the "Alléger" button shows "Alléger à 1 min"
