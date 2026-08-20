# id: ritual
# context: HabitManagement
# origin: issue-13
@feature:ritual
Feature: Practise a habit for its own dose

  @scenario:S1
  Scenario: Opening the ritual counts down the habit's own goal
    Given an active habit whose goal is 5 minutes
    When the user opens its ritual
    Then the countdown starts at its goal, not at a fixed minute

  @scenario:S3
  Scenario: Stopping early costs nothing
    Given a ritual in progress
    When the user stops it
    Then the screen returns to the detail and nothing is recorded, because stopping is not failing

  @scenario:S4
  Scenario: A habit at rest offers no practice
    Given a paused or anchored habit
    When its ritual address is reached by hand
    Then the practice is not offered, because a pause is real rest and an anchored habit has left the daily list

  @scenario:S5
  Scenario: An unknown habit lands quietly
    Given an address naming a habit that is not on the list
    When the ritual is opened
    Then the screen says the habit is no longer on the list and offers the way back

  @scenario:S6
  Scenario: Time away is time spent
    Given a ritual in progress
    When the screen is hidden and comes back later
    Then the countdown reflects the time that really elapsed, neither frozen nor drifted
