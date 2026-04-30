Feature: HL7 v2 Data Type Validation
  As an HL7 message processor
  I want to validate HL7 v2 data type values
  So that I can ensure messages contain properly formatted data

  # ===========================================================================
  # DataType Enum Parsing
  # ===========================================================================

  Scenario Outline: Parse known data type codes
    Given a data type code "<code>"
    When I parse the data type code
    Then the parsed data type should be <variant>

    Examples:
      | code | variant |
      | ST   | ST      |
      | ID   | ID      |
      | IS   | IS      |
      | DT   | DT      |
      | TM   | TM      |
      | TS   | TS      |
      | NM   | NM      |
      | SI   | SI      |
      | TX   | TX      |
      | FT   | FT      |
      | PN   | PN      |
      | CX   | CX      |
      | HD   | HD      |
      | AD   | AD      |
      | XTN  | XTN     |

  Scenario Outline: Reject unknown data type codes
    Given a data type code "<code>"
    When I parse the data type code
    Then the parsed data type should be None

    Examples:
      | code    |
      | INVALID |
      |         |
      | st      |
      | XX      |

  # ===========================================================================
  # ST (String Data)
  # ===========================================================================

  Scenario Outline: Validate ST (String Data) values
    Given a value "<value>"
    When I validate it as data type "ST"
    Then the validation result should be <result>

    Examples:
      | value       | result |
      | any string  | valid  |
      |             | valid  |
      | test123!@#  | valid  |

  # ===========================================================================
  # ID (Identifier)
  # ===========================================================================

  Scenario Outline: Validate ID (Identifier) values
    Given a value "<value>"
    When I validate it as data type "ID"
    Then the validation result should be <result>

    Examples:
      | value         | result  |
      | ABC123        | valid   |
      | test-value    | valid   |

  Scenario: Reject ID with control characters
    Given a value with embedded newline "test\nvalue"
    When I validate it as data type "ID"
    Then the validation result should be invalid

  # ===========================================================================
  # IS (Coded Value)
  # ===========================================================================

  Scenario Outline: Validate IS (Coded Value) values
    Given a value "<value>"
    When I validate it as data type "IS"
    Then the validation result should be <result>

    Examples:
      | value | result |
      | CODE1 | valid  |
      | 123   | valid  |

  # ===========================================================================
  # DT (Date)
  # ===========================================================================

  Scenario Outline: Validate DT (Date) values
    Given a value "<value>"
    When I validate it as data type "DT"
    Then the validation result should be <result>

    Examples:
      | value    | result  |
      | 20250128 | valid   |
      | 20251328 | invalid |
      | invalid  | invalid |

  # ===========================================================================
  # TM (Time)
  # ===========================================================================

  Scenario Outline: Validate TM (Time) values
    Given a value "<value>"
    When I validate it as data type "TM"
    Then the validation result should be <result>

    Examples:
      | value  | result  |
      | 152312 | valid   |
      | 1523   | valid   |
      | 252300 | invalid |

  # ===========================================================================
  # TS (Timestamp)
  # ===========================================================================

  Scenario Outline: Validate TS (Timestamp) values
    Given a value "<value>"
    When I validate it as data type "TS"
    Then the validation result should be <result>

    Examples:
      | value          | result  |
      | 20250128152312 | valid   |
      | 20250128       | valid   |
      | invalid        | invalid |

  # ===========================================================================
  # NM (Numeric)
  # ===========================================================================

  Scenario Outline: Validate NM (Numeric) values
    Given a value "<value>"
    When I validate it as data type "NM"
    Then the validation result should be <result>

    Examples:
      | value    | result  |
      | 123      | valid   |
      | 123.45   | valid   |
      | -123     | valid   |
      | 0        | valid   |
      | -123.456 | valid   |
      | abc      | invalid |
      | 12.34.56 | invalid |

  # ===========================================================================
  # SI (Sequence ID)
  # ===========================================================================

  Scenario Outline: Validate SI (Sequence ID) values
    Given a value "<value>"
    When I validate it as data type "SI"
    Then the validation result should be <result>

    Examples:
      | value | result  |
      | 1     | valid   |
      | 123   | valid   |
      | 0     | invalid |
      | -1    | invalid |
      | abc   | invalid |

  # ===========================================================================
  # TX (Text Data)
  # ===========================================================================

  Scenario Outline: Validate TX (Text Data) values
    Given a value "<value>"
    When I validate it as data type "TX"
    Then the validation result should be <result>

    Examples:
      | value    | result |
      | any text | valid  |
      |          | valid  |

  # ===========================================================================
  # FT (Formatted Text)
  # ===========================================================================

  Scenario Outline: Validate FT (Formatted Text) values
    Given a value "<value>"
    When I validate it as data type "FT"
    Then the validation result should be <result>

    Examples:
      | value          | result |
      | formatted text | valid  |
      |                | valid  |

  # ===========================================================================
  # PN (Person Name)
  # ===========================================================================

  Scenario Outline: Validate PN (Person Name) values
    Given a value "<value>"
    When I validate it as data type "PN"
    Then the validation result should be <result>

    Examples:
      | value      | result  |
      | Smith^John | valid   |
      | O'Brien^Mary | valid |
      | Doe-Jane   | valid   |
      | Dr. Smith  | valid   |
      | Smith123   | invalid |

  # ===========================================================================
  # CX (Extended Composite ID)
  # ===========================================================================

  Scenario Outline: Validate CX (Extended Composite ID) values
    Given a value "<value>"
    When I validate it as data type "CX"
    Then the validation result should be <result>

    Examples:
      | value   | result |
      | 12345   | valid  |
      | ABC-123 | valid  |

  # ===========================================================================
  # HD (Hierarchic Designator)
  # ===========================================================================

  Scenario Outline: Validate HD (Hierarchic Designator) values
    Given a value "<value>"
    When I validate it as data type "HD"
    Then the validation result should be <result>

    Examples:
      | value      | result |
      | HOSPITAL.1 | valid  |
      | FACILITY   | valid  |

  # ===========================================================================
  # AD (Address)
  # ===========================================================================

  Scenario Outline: Validate AD (Address) values
    Given a value "<value>"
    When I validate it as data type "AD"
    Then the validation result should be <result>

    Examples:
      | value                 | result |
      | 123 Main St           | valid  |
      | Apt 4B, 456 Oak Ave   | valid  |

  Scenario: Reject AD with control characters
    Given a value with embedded newline "Line\nBreak"
    When I validate it as data type "AD"
    Then the validation result should be invalid

  # ===========================================================================
  # XTN (Phone Number)
  # ===========================================================================

  Scenario Outline: Validate XTN (Phone Number) values
    Given a value "<value>"
    When I validate it as data type "XTN"
    Then the validation result should be <result>

    Examples:
      | value            | result  |
      | 1234567          | valid   |
      | 1234567890       | valid   |
      | (555) 123-4567   | valid   |
      | 555-123-4567     | valid   |
      | 123              | invalid |
      | 1234567890123456 | invalid |

  # ===========================================================================
  # Unknown data types
  # ===========================================================================

  Scenario Outline: Accept any value for unknown data types
    Given a value "<value>"
    When I validate it as data type "<dtype>"
    Then the validation result should be valid

    Examples:
      | value    | dtype   |
      | anything | UNKNOWN |
      |          | XX      |

  # ===========================================================================
  # Email validation
  # ===========================================================================

  Scenario Outline: Validate email addresses
    Given a value "<value>"
    When I validate it as an email
    Then the validation result should be <result>

    Examples:
      | value                 | result  |
      | test@example.com      | valid   |
      | user.name@domain.org  | valid   |
      | user+tag@example.com  | valid   |
      | a@b.co                | valid   |
      | invalid               | invalid |
      | @example.com          | invalid |
      | test@                 | invalid |
      | test@example          | invalid |

  # ===========================================================================
  # SSN validation
  # ===========================================================================

  Scenario Outline: Validate SSN values
    Given a value "<value>"
    When I validate it as an SSN
    Then the validation result should be <result>

    Examples:
      | value        | result  |
      | 123-45-6789  | valid   |
      | 123456789    | valid   |
      | 123 45 6789  | valid   |
      | 000-45-6789  | invalid |
      | 666-45-6789  | invalid |
      | 900-45-6789  | invalid |
      | 123-00-6789  | invalid |
      | 123-45-0000  | invalid |
      | 123-45-678   | invalid |
      | 123-45-67890 | invalid |

  # ===========================================================================
  # Luhn checksum validation
  # ===========================================================================

  Scenario Outline: Validate Luhn checksum
    Given a value "<value>"
    When I validate it with Luhn checksum
    Then the validation result should be <result>

    Examples:
      | value                | result  |
      | 4532015112830366     | valid   |
      | 6011111111111117     | valid   |
      | 378282246310005      | valid   |
      | 4111111111111111     | valid   |
      | 4532015112830367     | invalid |
      | 1234567890123456     | invalid |
      | 4532-0151-1283-0366  | valid   |
      | 1                    | invalid |

  # ===========================================================================
  # DataTypeValidator builder
  # ===========================================================================

  Scenario: Validator with minimum length constraint
    Given a validator with min length 5
    When I validate value "abc" with the validator
    Then the validation result should be invalid
    When I validate value "abcde" with the validator
    Then the validation result should be valid

  Scenario: Validator with maximum length constraint
    Given a validator with max length 10
    When I validate value "abc" with the validator
    Then the validation result should be valid
    When I validate value "abcdeabcdef" with the validator
    Then the validation result should be invalid

  Scenario: Validator with min and max length constraints
    Given a validator with min length 3 and max length 10
    When I validate value "ab" with the validator
    Then the validation result should be invalid
    When I validate value "abcde" with the validator
    Then the validation result should be valid
    When I validate value "abcdeabcdef" with the validator
    Then the validation result should be invalid

  Scenario: Validator with regex pattern constraint
    Given a validator with pattern "^\d{3}$"
    When I validate value "123" with the validator
    Then the validation result should be valid
    When I validate value "abc" with the validator
    Then the validation result should be invalid
    When I validate value "1234" with the validator
    Then the validation result should be invalid

  Scenario: Validator with allowed values constraint
    Given a validator with allowed values "M,F,U"
    When I validate value "M" with the validator
    Then the validation result should be valid
    When I validate value "F" with the validator
    Then the validation result should be valid
    When I validate value "X" with the validator
    Then the validation result should be invalid

  Scenario: Validator with Luhn checksum constraint
    Given a validator with Luhn checksum
    When I validate value "4532015112830366" with the validator
    Then the validation result should be valid
    When I validate value "4532015112830367" with the validator
    Then the validation result should be invalid

  Scenario: Validator with combined constraints
    Given a validator with min length 16 and max length 16 and Luhn checksum
    When I validate value "4532015112830366" with the validator
    Then the validation result should be valid
    When I validate value "453201511283036" with the validator
    Then the validation result should be invalid
    When I validate value "4532015112830367" with the validator
    Then the validation result should be invalid

  # ===========================================================================
  # Detailed validation errors
  # ===========================================================================

  Scenario: Detailed error for too short value
    Given a validator with min length 5
    When I validate value "abc" with detailed errors
    Then the error should be TooShort with length 3 and min 5

  Scenario: Detailed error for too long value
    Given a validator with max length 5
    When I validate value "abcdefgh" with detailed errors
    Then the error should be TooLong with length 8 and max 5

  Scenario: Detailed error for pattern mismatch
    Given a validator with pattern "^\d+$"
    When I validate value "abc123" with detailed errors
    Then the error should be PatternMismatch

  Scenario: Detailed error for not in allowed set
    Given a validator with allowed values "A,B"
    When I validate value "C" with detailed errors
    Then the error should be NotInAllowedSet with value "C"

  Scenario: Detailed error for checksum failure
    Given a validator with Luhn checksum
    When I validate value "1234567890123456" with detailed errors
    Then the error should be ChecksumFailed

  # ===========================================================================
  # Birth date validation
  # ===========================================================================

  Scenario: Validate a past birth date
    Given a birth date in the past
    When I validate it as a birth date
    Then the validation result should be valid

  Scenario: Reject a future birth date
    Given a birth date in the future
    When I validate it as a birth date
    Then the validation result should be invalid

  Scenario: Reject an invalid birth date string
    Given a value "invalid"
    When I validate it as a birth date
    Then the validation result should be invalid

  # ===========================================================================
  # Age range validation
  # ===========================================================================

  Scenario Outline: Validate age ranges
    Given a birth date "<birth>" and reference date "<reference>"
    When I validate the age range
    Then the validation result should be <result>

    Examples:
      | birth    | reference | result  |
      | 19900101 | 20250128  | valid   |
      | 20250101 | 20250101  | valid   |
      | 20250128 | 19900101  | invalid |

  Scenario: Reject age range with invalid dates
    Given a birth date "invalid" and reference date "20250128"
    When I validate the age range
    Then the validation result should be invalid

  # ===========================================================================
  # Numeric range validation
  # ===========================================================================

  Scenario Outline: Validate numeric ranges
    Given a value "<value>" with range "<min>" to "<max>"
    When I validate the numeric range
    Then the validation result should be <result>

    Examples:
      | value | min | max | result  |
      | 5     | 1   | 10  | valid   |
      | 1     | 1   | 10  | valid   |
      | 10    | 1   | 10  | valid   |
      | 0     | 1   | 10  | invalid |
      | 11    | 1   | 10  | invalid |
      | abc   | 1   | 10  | invalid |

  # ===========================================================================
  # Format matching
  # ===========================================================================

  Scenario Outline: Validate date format YYYY-MM-DD
    Given a value "<value>"
    When I validate it matches format "YYYY-MM-DD" for data type "DT"
    Then the validation result should be <result>

    Examples:
      | value      | result  |
      | 2025-01-28 | valid   |
      | 1900-12-31 | valid   |
      | 2025-13-28 | invalid |
      | 2025-01-32 | invalid |
      | 2025/01/28 | invalid |
      | 20250128   | invalid |

  Scenario Outline: Validate time format HH:MM:SS
    Given a value "<value>"
    When I validate it matches format "HH:MM:SS" for data type "TM"
    Then the validation result should be <result>

    Examples:
      | value    | result  |
      | 15:23:12 | valid   |
      | 00:00:00 | valid   |
      | 23:59:59 | valid   |
      | 24:00:00 | invalid |
      | 23:60:00 | invalid |
      | 23:59:60 | invalid |
      | 152312   | invalid |

  Scenario: Unknown format is accepted
    Given a value "anything"
    When I validate it matches format "UNKNOWN" for data type "ST"
    Then the validation result should be valid
