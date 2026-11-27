# id: system-back
# context: Navigation
# origin: S17 (FU1)
@feature:system-back
Feature: Android Back goes up one level, and leaves the app from Aujourd'hui

  Back follows the screen hierarchy, not the order the screens were opened in:
  a ritual goes back to its habit's detail, every other screen goes back to
  Aujourd'hui. Aujourd'hui has no parent, so Back there leaves the app. The
  native shell asks the page, and anything but a clear « handled » answer hands
  Back to the system (adr-0027).

  @scenario:S1
  Scenario Outline: Back goes up one level
    Given the user is on <screen>
    When the user presses Android Back
    Then <parent> is shown

    Examples:
      | screen                     | parent                   |
      | the ritual of a habit      | the detail of that habit |
      | the detail of a habit      | Aujourd'hui              |
      | the add-habit screen       | Aujourd'hui              |
      | Cette semaine              | Aujourd'hui              |
      | Ancrées                    | Aujourd'hui              |
      | the paused habits          | Aujourd'hui              |
      | a page that does not exist | Aujourd'hui              |

  @scenario:S2
  Scenario: Back on Aujourd'hui leaves the app
    Given the user is on Aujourd'hui
    When the user presses Android Back
    Then the app hands Back to the system, which leaves the app
