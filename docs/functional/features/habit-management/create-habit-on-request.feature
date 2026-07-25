# id: create-habit-on-request
# context: HabitManagement
# origin: F-2
@feature:create-habit-on-request
Feature: Create the habit from an accepted request

  @scenario:S1
  Scenario: Handling an accepted request creates the habit
    Given a published HabitRequested fact
    When the fact is handled
    Then the habit exists with the same id, title and goal
    And handling never fails on a habit rule, because the board already validated the request

  @scenario:S2
  Scenario: A requested habit ends up persisted end to end
    Given a habit board holding fewer than 5 habits
    When a habit is requested and the resulting fact is handled
    Then the persisted habit is the requested one
