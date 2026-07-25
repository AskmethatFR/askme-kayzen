# id: mark-done
# context: HabitManagement
# origin: slice-2
@feature:mark-done
Feature: Mark a habit done today

  @scenario:S1
  Scenario: Marking a habit records today's completion
    Given a habit with no completion for today
    When the user marks it done
    Then today's local date is recorded in its completion history

  @scenario:S2
  Scenario: Marking an already done habit clears today's completion
    Given a habit already marked done today
    When the user marks it done again
    Then today's completion is removed, because the gesture is a same-day toggle

  @scenario:S3
  Scenario: Marking an unknown habit is rejected
    Given a habit id that matches no habit
    When the user marks it done
    Then the gesture is rejected and no completion is recorded
