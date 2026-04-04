Feature: HL7 v2 Date/Time Parsing and Validation
  As an HL7 message processor
  I want to parse and validate HL7 date, time, and timestamp fields
  So that I can correctly handle temporal data in HL7 v2 messages

  # ---------------------------------------------------------------------------
  # Date (DT) Parsing
  # ---------------------------------------------------------------------------

  Scenario: Parse a valid HL7 date
    Given the date string "20250128"
    When I parse the date
    Then the year should be 2025
    And the month should be 1
    And the day should be 28

  Scenario: Parse a leap year date
    Given the date string "20240229"
    When I parse the date
    Then the year should be 2024
    And the month should be 2
    And the day should be 29

  Scenario: Reject an invalid date with bad month
    Given the date string "20251301"
    When I attempt to parse the date
    Then date parsing should fail

  Scenario: Reject a non-leap-year Feb 29
    Given the date string "20250229"
    When I attempt to parse the date
    Then date parsing should fail

  # ---------------------------------------------------------------------------
  # Time (TM) Parsing
  # ---------------------------------------------------------------------------

  Scenario: Parse a time with hour and minute only
    Given the time string "1523"
    When I parse the time
    Then the hour should be 15
    And the minute should be 23
    And the second should be 0
    And the fractional seconds should be absent

  Scenario: Parse a time with seconds
    Given the time string "152312"
    When I parse the time
    Then the hour should be 15
    And the minute should be 23
    And the second should be 12
    And the fractional seconds should be absent

  Scenario: Parse a time with fractional seconds
    Given the time string "152312.1234"
    When I parse the time
    Then the hour should be 15
    And the minute should be 23
    And the second should be 12
    And the fractional seconds should be 123400

  Scenario: Reject a time with invalid hour
    Given the time string "2500"
    When I attempt to parse the time
    Then time parsing should fail

  Scenario: Reject a time with invalid minute
    Given the time string "2360"
    When I attempt to parse the time
    Then time parsing should fail

  # ---------------------------------------------------------------------------
  # Timestamp (TS) Parsing
  # ---------------------------------------------------------------------------

  Scenario: Parse a full timestamp with seconds
    Given the timestamp string "20250128152312"
    When I parse the timestamp
    Then the parsed datetime year should be 2025
    And the parsed datetime month should be 1
    And the parsed datetime day should be 28
    And the parsed datetime hour should be 15
    And the parsed datetime minute should be 23
    And the parsed datetime second should be 12

  Scenario: Parse a date-only timestamp defaulting to midnight
    Given the timestamp string "20250128"
    When I parse the timestamp
    Then the parsed datetime year should be 2025
    And the parsed datetime hour should be 0
    And the parsed datetime minute should be 0
    And the parsed datetime second should be 0

  Scenario: Reject a timestamp that is too short
    Given the timestamp string "2025"
    When I attempt to parse the timestamp
    Then timestamp parsing should fail

  # ---------------------------------------------------------------------------
  # Timestamp Precision
  # ---------------------------------------------------------------------------

  Scenario Outline: Detect timestamp precision from input length
    Given the timestamp string "<input>"
    When I parse the timestamp with precision
    Then the precision should be "<precision>"

    Examples:
      | input                   | precision        |
      | 2025                    | Year             |
      | 202501                  | Month            |
      | 20250128                | Day              |
      | 2025012815              | Hour             |
      | 202501281523            | Minute           |
      | 20250128152312          | Second           |
      | 20250128152312.123456   | FractionalSecond |

  # ---------------------------------------------------------------------------
  # Timestamp Comparison
  # ---------------------------------------------------------------------------

  Scenario: An earlier timestamp is before a later one
    Given a timestamp "20250128100000" as the first
    And a timestamp "20250128120000" as the second
    Then the first timestamp should be before the second
    And the second timestamp should be after the first

  Scenario: Two timestamps on the same date are on the same day
    Given a timestamp "20250128" as the first
    And a timestamp "20250128235959" as the second
    Then the two timestamps should be on the same day

  Scenario: Two timestamps on different dates are not on the same day
    Given a timestamp "20250128" as the first
    And a timestamp "20250129000000" as the second
    Then the two timestamps should not be on the same day

  # ---------------------------------------------------------------------------
  # to_hl7_string Round-Trip
  # ---------------------------------------------------------------------------

  Scenario Outline: Round-trip a timestamp through to_hl7_string
    Given the timestamp string "<input>"
    When I parse the timestamp with precision
    And I format the timestamp to an HL7 string
    Then the HL7 string should be "<expected>"

    Examples:
      | input                   | expected                |
      | 2025                    | 2025                    |
      | 202501                  | 202501                  |
      | 20250128                | 20250128                |
      | 2025012815              | 2025012815              |
      | 202501281523            | 202501281523            |
      | 20250128152312          | 20250128152312          |

  # ---------------------------------------------------------------------------
  # Validation Boolean Helpers
  # ---------------------------------------------------------------------------

  Scenario Outline: Validate HL7 date strings
    Given the date string "<input>"
    Then the date validity should be <valid>

    Examples:
      | input      | valid |
      | 20250128   | true  |
      | 20240229   | true  |
      | 20251301   | false |
      | 2025       | false |
      | abcdefgh   | false |

  Scenario Outline: Validate HL7 time strings
    Given the time string "<input>"
    Then the time validity should be <valid>

    Examples:
      | input        | valid |
      | 0000         | true  |
      | 2359         | true  |
      | 152312.123   | true  |
      | 2400         | false |
      | 12           | false |

  Scenario Outline: Validate HL7 timestamp strings
    Given the timestamp string "<input>"
    Then the timestamp validity should be <valid>

    Examples:
      | input              | valid |
      | 20250128           | true  |
      | 20250128152312     | true  |
      | 2025               | false |
      | 20251328           | false |

  # ---------------------------------------------------------------------------
  # Current Date/Time Helpers
  # ---------------------------------------------------------------------------

  Scenario: now_hl7 returns a 14-digit valid timestamp
    When I call now_hl7
    Then the result should be 14 digits long
    And the result should be a valid timestamp

  Scenario: today_hl7 returns an 8-digit valid date
    When I call today_hl7
    Then the result should be 8 digits long
    And the result should be a valid date
