# id: add-habit
# context: HabitManagement
# origin: F-1, F-2, slice-R1
@feature:add-habit
Feature: Add a habit to my daily life

  @scenario:S1
  Scenario: A valid habit is added to the daily life
    Given a daily life holding fewer than 5 habits
    When a habit is added with a valid title and a goal of at least 1 minute
    Then the habit exists, carrying the generated id, the title and the goal

  @scenario:S2
  Scenario: A goal above 5 minutes is accepted
    Given a daily life holding fewer than 5 habits
    When a habit is added with a goal of 12 minutes
    Then the habit is added, because the goal is a soft target with no upper ceiling

  @scenario:S3
  Scenario: A habit breaking a habit rule is rejected and nothing is stored
    Given a daily life holding fewer than 5 habits
    When a habit is added with a goal of 0, an empty title, or a title longer than 50 characters
    Then it is rejected with the violated rule
    And no habit is stored

  @scenario:S4
  Scenario: A sixth habit is refused on a full daily life
    Given a daily life already holding 5 habits
    When a sixth habit is added
    Then it is rejected as daily-life-full
    And no habit is stored, and the five already there are unchanged

  @scenario:S5
  Scenario: A title already in the daily life is refused as a duplicate
    Given a daily life holding a habit titled "Lire une page"
    When a habit is added with the title "  lire une page  "
    Then it is rejected as a duplicate, because case and surrounding whitespace are ignored
    And no habit is stored

  @scenario:S6
  Scenario: A duplicate on a full daily life is reported as a duplicate, not as full
    Given a daily life holding 5 habits, one of them titled "Lire une page"
    When a habit is added with the title "Lire une page"
    Then it is rejected as a duplicate rather than as daily-life-full
