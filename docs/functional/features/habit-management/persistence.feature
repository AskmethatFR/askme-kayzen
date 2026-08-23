# id: persistence
# context: HabitManagement
# origin: issue-34
@feature:persistence
Feature: A habit persists across app restarts

  @scenario:S1
  Scenario: A habit added through the app is still there after a restart
    Given the app is launched with an empty store
    When a habit is added with a title and a goal
    And the app is closed and relaunched
    Then the habit is listed in the daily life

  @scenario:S2
  Scenario: A completion recorded is still there after a restart
    Given the app is launched with one habit in storage
    And that habit is marked done
    When the app is closed and relaunched
    Then the completion history carries the mark for today

  @scenario:S3
  Scenario: A first launch shows an empty daily life, with no seed
    Given the app is launched for the first time
    Then the board shows an empty daily life
    And the habit list is empty

  @scenario:S4
  Scenario: Unreadable stored data leaves an empty board, never a crash
    Given the app was launched with an unreadable or versioned-out stored state
    When the app is launched again
    Then the board shows an empty daily life
    And the unreadable data is set aside
    And no crash occurs

  @scenario:S5
  Scenario: No durable place to store habits refuses to start, loudly
    Given the platform offers no durable place to store habits
    When the app is launched
    Then a calm explanation is shown instead of the board
    And nothing is written to disk
