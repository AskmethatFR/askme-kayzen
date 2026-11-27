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
    Then the summary carries the habit id, its title and its goal in times a day
    And it is reported as not done today while it has no attempt today

  @scenario:S3
  Scenario: A habit that reached its goal today is reported done
    Given a habit whose attempts today have reached its goal
    When the Today screen asks for its habits
    Then the summary reports it done today, read from the day's attempt count

  @scenario:S4
  Scenario: An empty board shows the invitation and its only gesture
    Given a board with no habit
    When the Today screen renders
    Then the board displays the empty-state greeting
    And it shows the call to action « Rien pour l'instant. Et c'est très bien. Une seule toute petite habitude suffit pour commencer. »
    And the add-habit gesture is the only interactive element in the screen's content
    And the day's tally and the habit-list heading are hidden
    And no link to the Week screen is offered, the bottom bar carrying it

  @scenario:S5
  Scenario: Aujourd'hui hands the paused habits over to their own screen
    Given a board holding one active habit and one habit in pause
    When the Today screen renders
    Then it lists the active habit only
    And it no longer shows the paused zone « En pause · aucune pression »
    And it offers a link to the paused screen naming the paused count

  @scenario:S6
  Scenario: No paused link is offered when nothing is in pause
    Given a board holding one active habit
    When the Today screen renders
    Then no link to the paused screen is offered

  @scenario:S7
  Scenario: A board whose only habits are in pause keeps the invitation to add
    Given a board holding one habit in pause and nothing active
    When the Today screen renders
    Then the summary card and the habit-list heading are hidden
    And it offers a link to the paused screen
    And it offers the add-habit gesture

  @scenario:S8
  Scenario: The masthead names the day it is
    Given today is Saturday 19 September 2026
    When the Today screen renders
    Then the masthead reads « Samedi 19 septembre »
    And it does not read « Aujourd'hui »

  @scenario:S9
  Scenario Outline: Today shows a habit as done only once its day count has reached its goal
    Given a habit with a goal of 3 times a day
    And <attempts> attempts recorded today
    When the Today screen renders
    Then the habit's goal reads « 3 fois par jour »
    And the habit is shown as <state>
    And no count of attempts is shown for the habit
    And no alarm style marks the habit

    Examples:
      | attempts | state    |
      | 0        | not done |
      | 1        | not done |
      | 3        | done     |
      | 5        | done     |

  @scenario:S10
  Scenario: The tally counts the habits that reached their goal
    Given one active habit that reached its goal today and one that did not
    When the Today screen renders
    Then the tally reads « 1 sur 2 »

  @scenario:S11
  Scenario: A tap anywhere on a habit's card opens its detail, the target keeping its own tap
    Given a board holding one active habit
    When the Today screen renders
    Then the habit's title link covers the whole card with its invisible layer
    And the round target stays above that layer, so a tap on it marks the habit and does not open the detail

  @scenario:S12
  Scenario: With nothing done today the tally says nothing is required
    Given one active habit not yet done today
    When the Today screen renders
    Then the tally reads « Rien d'obligatoire aujourd'hui. »
