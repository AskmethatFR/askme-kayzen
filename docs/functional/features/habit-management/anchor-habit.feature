# id: anchor-habit
# context: HabitManagement
# origin: slice-6
@feature:anchor-habit
Feature: Anchor a habit that has become natural

  @wip @scenario:S1
  Scenario: Anchoring a habit frees a seat on the board
    Given a board holding 5 habits
    When the user anchors one of them
    Then a new habit can be requested and is accepted, because the board counts non-anchored habits only

  @wip @scenario:S2
  Scenario: An anchored habit is listed among the anchored ones
    Given an active habit
    When the user anchors it
    Then it leaves the Today list and is counted on the Ancrées screen

  @scenario:S3
  Scenario: An anchored habit can still be marked done
    Given an anchored habit
    When it is marked done
    Then today's completion is recorded, because anchoring ends the seat, not the habit

  @scenario:S4
  Scenario: Anchoring is only ever a user gesture
    Given a habit completed on 10 of the last 14 days
    When the user opens its detail
    Then nothing suggests anchoring it, because anchoring is user-initiated
