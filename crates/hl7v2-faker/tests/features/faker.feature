Feature: Realistic HL7 v2 Test Data Generation
  As a test engineer
  I want to generate realistic fake healthcare data
  So that I can build representative HL7 test messages

  # ============================================================================
  # Name Generation
  # ============================================================================

  Scenario: Generate a male name in HL7 format
    Given a faker with seed 42
    When I generate a name with gender "M"
    Then the name should be in "LAST^FIRST" format
    And the first name should be a known male name

  Scenario: Generate a female name in HL7 format
    Given a faker with seed 42
    When I generate a name with gender "F"
    Then the name should be in "LAST^FIRST" format
    And the first name should be a known female name

  Scenario: Generate a name with no gender specified
    Given a faker with seed 42
    When I generate a name with no gender
    Then the name should be in "LAST^FIRST" format

  Scenario: Name generation is deterministic with same seed
    Given a faker with seed 42
    When I generate a name with gender "M"
    And I generate another name with gender "M" using seed 42
    Then both names should be identical

  # ============================================================================
  # Address Generation
  # ============================================================================

  Scenario: Generate an address in HL7 format
    Given a faker with seed 42
    When I generate an address
    Then the address should contain component separators
    And the address should end with "USA"
    And the address should have a street number between 100 and 9999

  # ============================================================================
  # Phone Number Generation
  # ============================================================================

  Scenario: Generate a phone number
    Given a faker with seed 42
    When I generate a phone number
    Then the phone number should match the format "(XXX)XXX-XXXX"
    And the phone number should be 13 characters long

  # ============================================================================
  # SSN Generation
  # ============================================================================

  Scenario: Generate a Social Security Number
    Given a faker with seed 42
    When I generate an SSN
    Then the SSN should match the format "XXX-XX-XXXX"
    And the SSN should contain exactly 9 digits

  # ============================================================================
  # MRN Generation
  # ============================================================================

  Scenario: Generate a Medical Record Number
    Given a faker with seed 42
    When I generate an MRN
    Then the MRN should be between 6 and 10 digits long
    And the MRN should contain only digits

  # ============================================================================
  # Medical Code Generation
  # ============================================================================

  Scenario: Generate an ICD-10 code
    Given a faker with seed 42
    When I generate an ICD-10 code
    Then the ICD-10 code should match the format "LNN.N"
    And the ICD-10 code should start with an uppercase letter

  Scenario: Generate a LOINC code
    Given a faker with seed 42
    When I generate a LOINC code
    Then the LOINC code should be between 5 and 7 digits long
    And the LOINC code should contain only digits

  # ============================================================================
  # Medication and Allergen Generation
  # ============================================================================

  Scenario: Generate a medication name
    Given a faker with seed 42
    When I generate a medication name
    Then the medication should be a known medication

  Scenario: Generate an allergen name
    Given a faker with seed 42
    When I generate an allergen name
    Then the allergen should be a known allergen

  # ============================================================================
  # Patient Demographics
  # ============================================================================

  Scenario: Generate a blood type
    Given a faker with seed 42
    When I generate a blood type
    Then the blood type should be a valid ABO/Rh type

  Scenario: Generate an ethnicity
    Given a faker with seed 42
    When I generate an ethnicity
    Then the ethnicity should be a recognized value

  Scenario: Generate a race
    Given a faker with seed 42
    When I generate a race
    Then the race should be a recognized value

  # ============================================================================
  # Numeric Generation
  # ============================================================================

  Scenario: Generate a numeric string of specified length
    Given a faker with seed 42
    When I generate a numeric string of 8 digits
    Then the numeric string should be exactly 8 characters long
    And the numeric string should contain only digits

  Scenario: Generate a zero-length numeric string
    Given a faker with seed 42
    When I generate a numeric string of 0 digits
    Then the numeric string should be exactly 0 characters long

  # ============================================================================
  # Date Generation
  # ============================================================================

  Scenario: Generate a date within a valid range
    Given a faker with seed 42
    When I generate a date between "20200101" and "20201231"
    Then the date should be 8 digits in YYYYMMDD format
    And the date should be between "20200101" and "20201231"

  Scenario: Generate a date with identical start and end
    Given a faker with seed 42
    When I generate a date between "20200601" and "20200601"
    Then the date should be "20200601"

  Scenario: Generate a date with invalid start format
    Given a faker with seed 42
    When I generate a date between "invalid" and "20201231"
    Then the date generation should fail with an invalid format error

  # ============================================================================
  # Gaussian Distribution
  # ============================================================================

  Scenario: Generate a Gaussian distributed value
    Given a faker with seed 42
    When I generate a Gaussian value with mean 100.0 stddev 10.0 precision 2
    Then the Gaussian value should be a valid number
    And the Gaussian value should be within 5 standard deviations of the mean

  # ============================================================================
  # UUID Generation
  # ============================================================================

  Scenario: Generate a UUID v4
    Given a faker with seed 42
    When I generate a UUID v4
    Then the UUID should be 36 characters long
    And the UUID should have dashes at positions 8, 13, 18, and 23

  # ============================================================================
  # Timestamp Generation
  # ============================================================================

  Scenario: Generate a current UTC timestamp
    Given a faker with seed 42
    When I generate a UTC timestamp
    Then the timestamp should be 14 digits in YYYYMMDDHHMMSS format
    And the timestamp should start with "202"

  # ============================================================================
  # Selection Functions
  # ============================================================================

  Scenario: Select from a list of options
    Given a faker with seed 42
    When I select from the list "apple,banana,cherry"
    Then the selected value should be one of "apple,banana,cherry"

  Scenario: Select from an empty list
    Given a faker with seed 42
    When I select from an empty list
    Then the selection should return nothing

  Scenario: Select from a key-value map
    Given a faker with seed 42
    When I select from a map with entries "k1=v1,k2=v2,k3=v3"
    Then the selected value should be one of the map values "v1,v2,v3"

  Scenario: Select from an empty map
    Given a faker with seed 42
    When I select from an empty map
    Then the map selection should return nothing

  # ============================================================================
  # FakerValue Enum Generation
  # ============================================================================

  Scenario: FakerValue::Fixed returns the fixed string
    Given a faker with seed 42
    When I generate a FakerValue::Fixed with value "Hello HL7"
    Then the generated value should be "Hello HL7"

  Scenario: FakerValue::From selects from options
    Given a faker with seed 42
    When I generate a FakerValue::From with options "X,Y,Z"
    Then the generated value should be one of "X,Y,Z"

  Scenario: FakerValue::From with empty list returns error
    Given a faker with seed 42
    When I generate a FakerValue::From with no options
    Then the generation should fail with an empty options error

  Scenario: FakerValue::Numeric generates digits
    Given a faker with seed 42
    When I generate a FakerValue::Numeric with 5 digits
    Then the generated value should be 5 characters long
    And the generated value should contain only digits

  Scenario: FakerValue::RealisticName generates HL7 name
    Given a faker with seed 42
    When I generate a FakerValue::RealisticName for gender "M"
    Then the generated value should contain "^"

  Scenario: FakerValue::RealisticAddress generates HL7 address
    Given a faker with seed 42
    When I generate a FakerValue::RealisticAddress
    Then the generated value should contain "USA"

  Scenario: FakerValue::RealisticPhone generates phone number
    Given a faker with seed 42
    When I generate a FakerValue::RealisticPhone
    Then the generated value should start with "("

  Scenario: FakerValue::RealisticSsn generates SSN
    Given a faker with seed 42
    When I generate a FakerValue::RealisticSsn
    Then the generated value should be 11 characters long

  Scenario: FakerValue::RealisticMrn generates MRN
    Given a faker with seed 42
    When I generate a FakerValue::RealisticMrn
    Then the generated value length should be between 6 and 10

  Scenario: FakerValue::RealisticBloodType generates valid blood type
    Given a faker with seed 42
    When I generate a FakerValue::RealisticBloodType
    Then the generated value should be one of "A+,A-,B+,B-,AB+,AB-,O+,O-"

  Scenario: FakerValue::Map with empty map returns error
    Given a faker with seed 42
    When I generate a FakerValue::Map with no entries
    Then the generation should fail with an empty map error

  # ============================================================================
  # Determinism Across All Methods
  # ============================================================================

  Scenario: All faker methods are deterministic with the same seed
    Given a faker with seed 42
    When I generate all data types
    And I generate all data types again with seed 42
    Then both sets of generated data should be identical
