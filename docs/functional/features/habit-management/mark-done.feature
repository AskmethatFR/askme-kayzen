# id: mark-done
# context: HabitManagement
# origin: slice-2
@feature:mark-done
Feature: Mark a habit done today

  @scenario:S1
  Scenario: Marking a habit records today's first attempt
    Given a habit with no attempt for today
    When the user marks it done
    Then today's count of attempts is 1 and the habit is done today

  @scenario:S3
  Scenario: Marking an unknown habit is rejected
    Given a habit id that matches no habit
    When the user marks it done
    Then the gesture is rejected and no completion is recorded

  @scenario:S4
  Scenario: Saying a habit is done from its own screen
    Given an active habit with no completion for today
    When the user says it is done from the habit's own screen
    Then today's local date is recorded and one attempt is added, without entering the ritual

  @scenario:S5
  Scenario: Marking a habit again adds one more attempt, never clears the day
    Given a habit with 1 attempt today
    When the user marks it done again
    Then today's count of attempts is 2 and the habit is still done today

  @scenario:S6
  Scenario Outline: Attempts past the goal are counted, never refused
    Given a habit whose goal is 2 times per day, with <before> attempts today
    When the user marks it done
    Then the gesture is accepted and today's count of attempts is <after>

    Examples:
      | before | after |
      | 2      | 3     |
      | 4      | 5     |

  @scenario:S7
  Scenario: A day's count belongs to one habit and one local date
    Given two habits, the first with 1 attempt yesterday
    When the user marks the first habit done today
    Then the first habit counts 1 attempt today and still 1 yesterday, and the second counts none today
