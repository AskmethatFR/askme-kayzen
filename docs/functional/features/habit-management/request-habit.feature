# id: request-habit
# context: HabitManagement
# origin: F-1, slice-R1
@feature:request-habit
Feature: Request a habit from the board

  @scenario:S1
  Scenario: A valid request is accepted and published once
    Given a habit board holding fewer than 5 habits
    When a habit is requested with a valid title and a goal of at least 1 minute
    Then exactly one HabitRequested fact is published, carrying the generated id, the title and the goal
    And the board records the accepted request

  @scenario:S2
  Scenario: A goal above 5 minutes is accepted
    Given a habit board holding fewer than 5 habits
    When a habit is requested with a goal of 12 minutes
    Then the request is accepted, because the goal is a soft target with no upper ceiling

  @scenario:S3
  Scenario: A request breaking a habit rule is rejected and publishes nothing
    Given a habit board holding fewer than 5 habits
    When a habit is requested with a goal of 0, an empty title, or a title longer than 50 characters
    Then the request is rejected with the violated rule
    And nothing is published

  @scenario:S4
  Scenario: A sixth habit is refused on a full board
    Given a habit board already holding 5 habits
    When a sixth habit is requested
    Then the request is rejected as board-full
    And nothing is published and the board is unchanged

  @scenario:S5
  Scenario: A title already on the board is refused as a duplicate
    Given a habit board holding a habit titled "Lire une page"
    When a habit is requested with the title "  lire une page  "
    Then the request is rejected as a duplicate, because case and surrounding whitespace are ignored
    And nothing is published

  @scenario:S6
  Scenario: A duplicate on a full board is reported as a duplicate, not as full
    Given a habit board holding 5 habits, one of them titled "Lire une page"
    When a habit is requested with the title "Lire une page"
    Then the request is rejected as a duplicate rather than as board-full
