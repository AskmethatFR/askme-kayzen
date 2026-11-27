# id: add-habit
# context: HabitManagement
# origin: F-1, F-2, slice-R1
@feature:add-habit
Feature: Add a habit to my daily life

  @scenario:S1
  Scenario: A valid habit is added to the daily life
    Given a daily life holding fewer than 5 habits
    When a habit is added with a valid title and a goal of at least 1 time
    Then the habit exists, carrying the generated id, the title and the goal

  @scenario:S2
  Scenario: A goal above 5 times is accepted
    Given a daily life holding fewer than 5 habits
    When a habit is added with a goal of 12 times
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

  @scenario:S7
  Scenario: A new habit starts at 3 times a day
    Given a daily life holding fewer than 5 habits
    When a habit is added through the add flow with no goal chosen
    Then the habit is stored with a goal of 3 times a day

  @scenario:S8
  Scenario: The add screen blocks gently on a full daily life and invites anchoring one
    Given a daily life holding 5 habits, paused ones included, none anchored
    When the add screen is opened
    Then no idea and no field are offered, only the invitation to anchor one first

  @scenario:S9
  Scenario: Submitting with no name says why and stores nothing
    Given a daily life holding fewer than 5 habits
    When the add form is submitted, by Enter or by the button, with no name
    Then the note "Donnez-lui un nom pour commencer" appears
    And no habit is stored

  @scenario:S10
  Scenario: Tapping an idea already in the daily life says so and adds nothing more
    Given a daily life holding fewer than 5 habits, one of them titled like an offered idea
    When that idea is tapped
    Then the note "Elle est déjà dans votre quotidien" appears
    And no habit is stored beyond the one already there

  @scenario:S11
  Scenario: The refusal note is a live region, present and empty before any refusal
    Given a daily life holding fewer than 5 habits
    When the add screen is opened
    Then exactly one status note is present and it is empty

  @scenario:S12
  Scenario: The 50-character limit counts characters, so accents never cut a title shorter
    Given a daily life holding fewer than 5 habits
    When a habit is added with a title of 50 accented characters
    Then the habit is added, because the limit counts characters and not bytes

  @scenario:S13
  Scenario: Control and bidi-override characters are stripped from a title
    Given a daily life holding fewer than 5 habits
    When a habit is added with a title holding a newline, a NUL and a right-to-left override
    Then the habit is stored with those characters removed, a tab or newline between words becoming a space

  @scenario:S14
  Scenario: A title of 51 accented characters is refused
    Given a daily life holding fewer than 5 habits
    When a habit is added with a title of 51 accented characters
    Then it is rejected with the title-length rule

  @scenario:S15
  Scenario: The add field stops typing at 50 characters
    Given a daily life holding fewer than 5 habits
    When the add screen is opened
    Then the name field carries a maximum length of 50 characters

  @scenario:S16
  Scenario: A title made only of control, bidi-override or invisible characters is refused
    Given a daily life holding fewer than 5 habits
    When a habit is added with a title made only of control, bidi-override or invisible characters, such as a zero-width space or a lone zero-width joiner
    Then it is rejected with the title-length rule
