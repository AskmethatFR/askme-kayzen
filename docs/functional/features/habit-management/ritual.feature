# id: ritual
# context: HabitManagement
# origin: issue-13
@feature:ritual
Feature: Practise a habit for its own dose

  @scenario:S1
  Scenario: Opening the ritual shows the habit as a counter with no duration
    Given an active habit whose goal is 5 times a day
    When the user opens its ritual
    Then the ritual shows the habit's title and the completion gesture, with no countdown and no minutes

  @scenario:S2
  Scenario: Completing the practice from the ritual
    Given a ritual in progress for an active habit
    When the user taps the completion gesture
    Then one attempt is recorded for today and the screen returns to Aujourd'hui

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

  @scenario:S7
  Scenario: The completion gesture adds an attempt, it never un-marks
    Given a habit that already has attempts today, even up to its goal
    When the ritual's completion gesture runs again
    Then one more attempt is recorded, because completing counts a practice, it does not toggle one
