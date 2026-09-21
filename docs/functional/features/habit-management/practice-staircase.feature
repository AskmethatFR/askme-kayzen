# id: practice-staircase
# context: HabitManagement
# origin: slice-3b
@feature:practice-staircase
Feature: Read a habit's recent practice as a staircase

  # The week of pebbles answers two questions with one drawing: am I keeping it
  # up (the run of pebbles) and, across those days, am I raising, easing or
  # holding my effort (their size). It draws PRACTICE, never intent — adjusting
  # the goal changes nothing until a day is actually done (lifecycle-backlog,
  # slice 3b). A practised day is filled, a day without practice keeps its
  # pebble in outline — neither a gap nor a warning — and today is told apart by
  # its dashed outline until it is practised.

  @scenario:S1
  Scenario: A day that was done draws a filled pebble
    Given a habit whose goal is 5 minutes
    When the user marks it done today
    Then today's pebble is filled, standing at 5 minutes

  @scenario:S2
  Scenario: A day that was not done draws the same pebble, in outline
    Given a habit that was not marked done yesterday
    When the user opens its detail
    Then yesterday's pebble is drawn in outline
    And it is neither a gap nor a warning, because a day without practice is not a failure

  @scenario:S3
  Scenario: Adjusting the goal draws nothing on its own
    Given a habit the user has not marked done today
    When the user chooses "grandir"
    Then no pebble changes, because the week draws practice and not intent

  @scenario:S4
  Scenario: Each pebble is sized to the goal that was active that day
    Given a habit done at 5 minutes one day, grown to 6 minutes, then done again the next day
    When the user opens its detail
    Then the earlier pebble is sized for 5 minutes and the later one for 6 minutes

  @scenario:S5
  Scenario: The week covers the last seven days
    Given a habit created three weeks ago
    When the user opens its detail
    Then seven pebbles are drawn, one for each of the last seven days

  @scenario:S6
  Scenario: A brand-new habit already has a week of pebbles
    Given a habit created today and not yet done
    When the user opens its detail
    Then seven pebbles are drawn in outline, because an empty start is still a start

  @scenario:S7
  Scenario: Today is told apart by the shape of its pebble
    Given a habit the user has not marked done today
    When the user opens its detail
    Then today's pebble is drawn as a dashed outline, and the other days as plain outlines
