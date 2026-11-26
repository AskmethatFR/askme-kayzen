# id: undo-attempt
# context: HabitManagement
# origin: slice-S6
@feature:undo-attempt
Feature: Undo the last attempt of today

  @scenario:S1
  Scenario: Undoing today's attempt lowers today's count by one
    Given a habit with a goal of 2 and 2 attempts today
    When the user removes one attempt
    Then today's count of attempts is 1, the day is still practised and the habit is not done today, because the count is below the goal

  @scenario:S2
  Scenario: Undoing the only attempt of today clears it
    Given a habit with 1 attempt today
    When the user removes one attempt
    Then today's count of attempts is 0, the habit is not done today and the day is not recorded

  @scenario:S3
  Scenario: Undoing with no attempt today changes nothing and says nothing
    Given a habit with no attempt for today
    When the user removes one attempt
    Then today's count of attempts stays 0 and no error is shown

  @scenario:S4
  Scenario: The undo control is shown only when today has an attempt
    Given an active habit shown on its own screen
    When today's count of attempts is 0 and then 1
    Then the undo control is hidden at 0 and shown at 1

  @scenario:S5
  Scenario: An unknown habit id is rejected and nothing changes
    Given a habit id that matches no habit
    When the user removes one attempt
    Then the gesture is rejected and no count changes
