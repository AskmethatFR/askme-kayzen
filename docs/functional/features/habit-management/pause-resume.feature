# id: pause-resume
# context: HabitManagement
# origin: slice-5
@feature:pause-resume
Feature: Pause a habit and resume it

  @wip @scenario:S1
  Scenario: Pausing a habit moves it out of the daily list
    Given an active habit on the board
    When the user pauses it
    Then it leaves the Today list and appears in the paused zone

  @wip @scenario:S2
  Scenario: Resuming a paused habit brings it back in one gesture
    Given a paused habit
    When the user resumes it
    Then it is active again and appears in the Today list
    And its completion history is untouched

  @wip @scenario:S3
  Scenario: A paused habit keeps its seat on the board
    Given a board holding 5 habits, one of them paused
    When a new habit is requested
    Then the request is rejected as board-full, because a paused habit keeps its seat so resuming can never fail
