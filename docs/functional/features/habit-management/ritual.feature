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

  @scenario:S2
  Scenario: Completing the practice from the ritual
    Given a ritual in progress for an active habit
    When the user taps the completion gesture
    Then the habit is recorded done for today and the screen returns to Aujourd'hui

  @scenario:S3
  Scenario: Stopping early costs nothing
    Given a ritual in progress
    When the user stops it
    Then the screen returns to the detail and nothing is recorded, because stopping is not failing

  @scenario:S4
  Scenario: A habit at rest offers no practice
    Given a paused or anchored habit
    When its ritual address is reached by hand
    Then neither the practice nor the completion gesture is offered, because a pause is real rest and an anchored habit has left the daily list

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

  @scenario:S7
  Scenario: The completion gesture states a fact, it never un-marks
    Given a habit already recorded done for today
    When the ritual's completion gesture runs again
    Then the day stays recorded, because completing asserts a practice, it does not toggle one

  @scenario:S8
  Scenario: At zero the screen says a gentle word, not a verdict
    Given a ritual whose countdown has reached zero
    When the screen is read
    Then it speaks a gentle word for the time given and still waits for the gesture, validating nothing on its own
