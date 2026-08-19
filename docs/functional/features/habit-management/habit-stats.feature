# id: habit-stats
# context: HabitManagement
# origin: slice-8
@feature:habit-stats
Feature: Read a habit's story

  # Everything below is derived from the completion and step histories on read —
  # nothing is stored (adr-0006-cqrs-light).

  @scenario:S1
  Scenario: The recap counts the days done and the other days
    Given a habit whose life spans 30 days, completed on 12 of them
    When the user opens its recap
    Then it reads "12 réalisés" and "18 autres jours"
    And the days without practice are never named a failure

  @scenario:S2
  Scenario: The recap counts how often the goal moved
    Given a habit grown 3 times and lightened once, now at 7 minutes
    When the user opens its recap
    Then it reads "3 fois grandie", "1 fois allégée" and the current goal
    And the lightening is never named a setback

  @scenario:S3
  Scenario: The recap sums the minutes practised since the beginning
    Given a habit completed on two days whose goal was 5 minutes, then on one day whose goal was 6
    When the user opens its recap
    Then it reads "16 minutes de pratique accumulées"
    And the label says time practised, never gain over the starting goal

  @scenario:S4
  Scenario: The recap message stays free of guilt whatever the history
    Given a habit with no completion for the last 10 days
    When the user opens its recap
    Then the message acknowledges the rest without blaming, because an empty day is never a failure

  @scenario:S5
  Scenario: A brand-new habit already has something to read
    Given a habit created today and not yet done
    When the user opens its recap
    Then the message reads "Un début parfait", because an empty start is still a start
