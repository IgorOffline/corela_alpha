@gui
Feature: Travel between regions
  A new session starts in Center at (0, 0). Moving north off Center's top
  cell enters North, and moving south from there returns to Center.

  Scenario: Move from Center to North and back to Center
    Given a new session that ends after 6 actions
    When I move north 5 times
    And I move south 1 time
    Then corela saves a replay of 6 actions
    And the replay shows
      | action | region | note                  |
      | 1      | Center | selected (0, -1)      |
      | 2      | Center | selected (0, -2)      |
      | 3      | Center | selected (0, -3)      |
      | 4      | Center | selected (0, -4)      |
      | 5      | North  | entered North region  |
      | 6      | Center | entered Center region |
