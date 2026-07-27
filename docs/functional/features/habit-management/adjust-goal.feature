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
    And the change is recorded in the step history with today's local date

  @scenario:S2
  Scenario: Lightening a habit lowers its goal by one minute
    Given a habit with a goal of 5 minutes
    When the user chooses "alléger"
    Then the goal becomes 4 minutes
    And the change is recorded in the step history with today's local date

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
