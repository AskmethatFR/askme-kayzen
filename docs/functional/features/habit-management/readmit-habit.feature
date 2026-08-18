# id: readmit-habit
# context: HabitManagement
# origin: slice-7
@feature:readmit-habit
Feature: Put an anchored habit back into the daily life

  @scenario:S1
  Scenario: Readmitting an anchored habit gives it a place in the daily life again
    Given an anchored habit and a daily life holding fewer than 5 non-anchored habits
    When the user chooses "la remettre dans mon quotidien"
    Then the habit is active again and takes a place in the daily life
    And its completion and step histories are untouched

  @scenario:S2
  Scenario: Readmission is refused when the daily life is full
    Given an anchored habit and a daily life already holding 5 non-anchored habits
    When the user chooses to readmit it
    Then the readmission is refused as daily-life-full and the habit stays anchored

  @scenario:S3
  Scenario: Readmission is refused when the title is already back in the daily life
    Given an anchored habit titled "Lire une page" and a daily life holding a habit titled "lire une page"
    When the user chooses to readmit it
    Then the readmission is refused as a duplicate and the habit stays anchored

  @scenario:S4
  Scenario: The Ancrées screen states how many habits are followed in parallel
    Given an anchored habit and a daily life holding 2 non-anchored habits
    When the user opens the Ancrées screen
    Then the screen reads "Vous suivez 2 / 5 habitudes en parallèle"
