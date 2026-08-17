# id: pause-resume
# context: HabitManagement
# origin: slice-5
@feature:pause-resume
Feature: Pause a habit and resume it

  @scenario:S1
  Scenario: Pausing a habit moves it out of the daily list
    Given an active habit on the board
    When the user pauses it
    Then it leaves the Today list and appears in the paused zone

  @scenario:S2
  Scenario: Resuming a paused habit brings it back in one gesture
    Given a paused habit
    When the user resumes it
    Then it is active again and appears in the Today list
    And its completion history is untouched

  @scenario:S3
  Scenario: A paused habit keeps its seat in the daily life
    Given a daily life holding 5 habits, one of them paused
    When a new habit is added
    Then it is rejected as daily-life-full, because a paused habit keeps its seat so resuming can never fail

  @scenario:S4
  Scenario: The detail of a paused habit offers only its return
    Given a paused habit
    When the user opens its detail
    Then the screen offers to resume it and shows its practice staircase
    And it offers neither the ritual, nor growing, nor lightening, because a pause is real rest
