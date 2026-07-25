# id: readmit-habit
# context: HabitManagement
# origin: slice-7
@feature:readmit-habit
Feature: Put an anchored habit back into the daily life

  @wip @scenario:S1
  Scenario: Readmitting an anchored habit gives it a seat again
    Given an anchored habit and a board holding fewer than 5 non-anchored habits
    When the user chooses "la remettre dans mon quotidien"
    Then the habit is active again and takes a seat on the board
    And its completion and step histories are untouched

  @wip @scenario:S2
  Scenario: Readmission is refused when the board is full
    Given an anchored habit and a board already holding 5 non-anchored habits
    When the user chooses to readmit it
    Then the readmission is refused as board-full and the habit stays anchored

  @wip @scenario:S3
  Scenario: Readmission is refused when the title is already back on the board
    Given an anchored habit titled "Lire une page" and a board holding a habit titled "lire une page"
    When the user chooses to readmit it
    Then the readmission is refused as a duplicate and the habit stays anchored
