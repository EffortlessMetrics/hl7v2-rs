Feature: Full-Text Search
  As a healthcare data analyst
  I want to search HL7 messages by content
  So that I can find relevant clinical data

  Background:
    Given the index contains these messages:
      | id      | message_type | patient_name | source    | content_snippet          |
      | MSG001  | ADT^A01      | John Doe     | Epic      | Patient admitted         |
      | MSG002  | ORU^R01      | Jane Smith   | LabCorp   | Glucose level 120        |
      | MSG003  | ADT^A08      | John Doe     | Epic      | Patient info updated     |
      | MSG004  | ORU^R01      | Bob Johnson  | Quest     | Cholesterol 180          |
      | MSG005  | ADT^A03      | Alice Brown  | Cerner    | Patient discharged       |

  Rule: Basic full-text search returns matching documents

    Example: Search by patient name
      When I search for "John Doe"
      Then I should get 2 results
      And the results should contain IDs "MSG001" and "MSG003"

    Example: Search by partial name
      When I search for "John"
      Then I should get at least 2 results
      And the results should contain patient name "John Doe"

    Example: Search by message content
      When I search for "Glucose"
      Then I should get 1 result
      And the first result should have ID "MSG002"

    Example: Search with no results
      When I search for "xyznonexistent"
      Then I should get 0 results
      And the total count should be 0

  Rule: Boolean operators combine search terms

    Example: AND operator requires both terms
      When I search for "John AND admitted"
      Then I should get 1 result
      And the first result should have ID "MSG001"

    Example: OR operator matches either term
      When I search for "Glucose OR Cholesterol"
      Then I should get 2 results
      And the results should contain IDs "MSG002" and "MSG004"

    Example: NOT operator excludes terms
      Given I search for all messages
      When I exclude "ORU^R01"
      Then I should get 3 results
      And the results should not contain IDs "MSG002" and "MSG004"

  Rule: Field-specific search targets specific data

    Example: Search by message type field
      When I search for "message_type:ADT^A01"
      Then I should get 1 result
      And the first result should have ID "MSG001"

    Example: Search by source field
      When I search for "source:Epic"
      Then I should get 2 results
      And all results should have source "Epic"

    Example: Search by patient ID
      When I search for "patient_id:MRN12345"
      Then I should get 1 result

    Example: Combined field and text search
      When I search for "message_type:ADT^A01 AND John"
      Then I should get 1 result
      And the first result should have ID "MSG001"

  Rule: Search results include relevance scoring

    Example: Results are ordered by relevance
      Given messages with varying relevance to "Glucose"
      When I search for "Glucose"
      Then results should be ordered by descending score
      And the highest score should be for exact matches

    Example: Relevance scores are between 0 and 1
      When I search for "Patient"
      Then all results should have scores between 0.0 and 1.0
