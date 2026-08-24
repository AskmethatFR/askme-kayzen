# id: language
# context: Localisation
# origin: issue-10
@feature:language
Feature: The app speaks the language of the device it runs on

  @scenario:S1
  Scenario: An English device is answered in English
    Given the device reports an English locale
    When the app is launched
    Then every screen reads in English

  @scenario:S2
  Scenario: A language the app does not carry falls back to French
    Given the device reports a locale the app carries no catalogue for
    When the app is launched
    Then every screen reads in French

  @scenario:S3
  Scenario: A device that reports no language at all falls back to French
    Given the device reports no locale
    When the app is launched
    Then every screen reads in French

  @scenario:S4
  Scenario: A screen never mixes two languages
    Given the device reports an English locale
    When any screen is opened
    Then no French copy remains anywhere on it
