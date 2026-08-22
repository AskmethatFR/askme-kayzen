# id: week-recap
# context: HabitManagement
# origin: issue-22
@feature:week-recap
Feature: Read the week's practice as accumulated minutes and rhythm

  # Everything below is derived from the completion and step histories on read —
  # nothing is stored (adr-0006-cqrs-light).

  @scenario:S1
  Scenario: The large figure sums minutes practised across every habit
    Given three habits active this week, completed on 3, 2, and 1 days at 5 minutes each
    When the user opens the week screen
    Then the large figure reads "30 minutes de pratique accumulées"
    And the label names accumulated practice, never gain over the starting goal

  @scenario:S2
  Scenario: Paused and anchored habits still count in the sum
    Given a habit paused on day 3 after 4 completed days, and an anchored habit with 3 completed days
    When the user opens the week screen
    Then their lived minutes still count in the large figure
    And each still reads its own journey as a row
    And pausing or anchoring never takes lived minutes back

  @scenario:S3
  Scenario: A week with no practice reads gently
    Given the week just began and nothing has been practised yet
    When the user opens the week screen
    Then the week's word reads "Un début parfait", because an empty start is still a start
    And the screen never states a bare "0" as a verdict

  @scenario:S4
  Scenario: A week without practice is acknowledged as rest
    Given a habit practised earlier, but not once in the last seven days
    When the user opens the week screen
    Then the week acknowledges the rest without blaming
    And the message never frames empty days as a failure

  @scenario:S5
  Scenario: Each habit shows its journey with one bar per goal step
    Given a habit grown from 3 to 5 minutes mid-week and completed on four days
    When the user opens the week screen
    Then that habit's row reads "3 → 5 min"
    And its mini-curve draws one bar per goal step, not one per day

  @scenario:S6
  Scenario: The rhythm keeps one dot per day, faint when no practice
    Given the last seven days, with practice on days 1, 3, and 5 only
    When the user opens the week screen
    Then the rhythm row shows seven dots, lit on practiced days and faint on others
    And a day without practice keeps its dot faint, never a gap, never red

  @scenario:S7
  Scenario: A brand-new habit already shows its journey
    Given a habit created today at 5 minutes and not yet practised
    When the user opens the week screen
    Then that habit's row reads "5 → 5 min" with a single bar
    And an empty start is still a start

  @scenario:S8
  Scenario: A habit practised in the rolling window reads its curve in the accent
    Given one habit practised at least once in the last seven days, and one not practised at all
    When the user opens the week screen
    Then the practised habit's mini-curve reads in the accent, which says "practised"
    And the unpractised habit's curve keeps the neutral tone, with nothing added
    And no counter, no mark of absence: the recap informs, it never reproaches
