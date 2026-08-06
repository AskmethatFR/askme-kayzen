# id: practice-staircase
# context: HabitManagement
# origin: slice-3b
@feature:practice-staircase
Feature: Read a habit's recent practice as a staircase

  # The staircase answers two questions with one drawing: am I keeping it up
  # (the run of bars) and, across those days, am I raising, easing or holding my
  # effort (their profile). It draws PRACTICE, never intent — adjusting the goal
  # changes nothing until a day is actually done (lifecycle-backlog, slice 3b).

  @scenario:S1
  Scenario: A day that was done draws a full bar
    Given a habit whose goal is 5 minutes
    When the user marks it done today
    Then today's bar is full, standing at 5 minutes

  @scenario:S2
  Scenario: A day that was not done draws the same bar, faintly
    Given a habit that was not marked done yesterday
    When the user opens its detail
    Then yesterday's bar is drawn faint
    And it is neither a gap nor a warning, because a day without practice is not a failure

  @scenario:S3
  Scenario: Adjusting the goal draws nothing on its own
    Given a habit the user has not marked done today
    When the user chooses "grandir"
    Then no bar changes, because the staircase draws practice and not intent

  @scenario:S4
  Scenario: Each bar stands at the goal that was active that day
    Given a habit done at 5 minutes one day, grown to 6 minutes, then done again the next day
    When the user opens its detail
    Then the earlier bar stands at 5 minutes and the later one at 6 minutes

  @scenario:S5
  Scenario: The staircase covers the last seven days
    Given a habit created three weeks ago
    When the user opens its detail
    Then seven bars are drawn, one for each of the last seven days

  @scenario:S6
  Scenario: A brand-new habit already has a staircase
    Given a habit created today and not yet done
    When the user opens its detail
    Then seven faint bars are drawn, because an empty start is still a start
