# id: habit-stats
# context: HabitManagement
# origin: slice-8
@feature:habit-stats
Feature: Read a habit's story

  # Everything below is derived from the completion and step histories on read —
  # nothing is stored (adr-0006-cqrs-light).

  @wip @scenario:S1
  Scenario: The recap counts the days done and the empty days
    Given a habit completed on 12 days since its creation, 30 days ago
    When the user opens its recap
    Then it reads 12 days done and 18 empty days, named as empty and never as failed

  @wip @scenario:S2
  Scenario: The recap counts how often the goal moved
    Given a habit grown 3 times and lightened once
    When the user opens its recap
    Then it reads 3 growths and 1 lightening, with the current goal

  @wip @scenario:S3
  Scenario: The recap sums the minutes gained since the beginning
    Given a habit completed on days whose goal was 5, then 6 minutes
    When the user opens its recap
    Then the minutes gained sum each completed day against the goal in force that day

  @wip @scenario:S4
  Scenario: The recap message stays free of guilt whatever the history
    Given a habit with no completion for the last 10 days
    When the user opens its recap
    Then the message acknowledges the pause without blaming, because an empty day is never a failure
