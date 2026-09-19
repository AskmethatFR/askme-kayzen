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
