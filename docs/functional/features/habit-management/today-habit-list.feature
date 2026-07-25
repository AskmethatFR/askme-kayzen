# id: today-habit-list
# context: HabitManagement
# origin: slice-1
@feature:today-habit-list
Feature: List the board habits for today

  @scenario:S1
  Scenario: An empty board shows no habit
    Given a board with no habit
    When the Today screen asks for its habits
    Then it receives an empty list

  @scenario:S2
  Scenario: A habit is summarised with its title and goal
    Given a board holding one habit
    When the Today screen asks for its habits
    Then the summary carries the habit id, its title and its goal in minutes
    And it is reported as not done today while no completion exists for today

  @scenario:S3
  Scenario: A habit completed today is reported done
    Given a habit already marked done today
    When the Today screen asks for its habits
    Then the summary reports it done today, read from the completion history
