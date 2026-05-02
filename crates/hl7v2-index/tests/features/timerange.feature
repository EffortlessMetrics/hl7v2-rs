Feature: Time-Range Queries
  As a system administrator
  I want to query messages by time range
  So that I can audit recent activity

  Background:
    Given the index contains messages with timestamps:
      | id      | timestamp            | message_type |
      | MSG001  | 2023-11-01T10:00:00Z | ADT^A01      |
      | MSG002  | 2023-11-15T14:30:00Z | ORU^R01      |
      | MSG003  | 2023-11-30T08:00:00Z | ADT^A08      |
      | MSG004  | 2023-12-05T16:45:00Z | ORU^R01      |
      | MSG005  | 2023-12-20T09:15:00Z | ADT^A03      |

  Rule: Time ranges filter messages by timestamp

    Example: Search within single day range
      When I query messages from "2023-11-15T00:00:00Z" to "2023-11-15T23:59:59Z"
      Then I should get 1 result
      And the result should have ID "MSG002"

    Example: Search within month range
      When I query messages from "2023-11-01T00:00:00Z" to "2023-11-30T23:59:59Z"
      Then I should get 3 results
      And the results should contain IDs "MSG001", "MSG002", and "MSG003"

    Example: Search with no time range matches
      When I query messages from "2024-01-01T00:00:00Z" to "2024-01-31T23:59:59Z"
      Then I should get 0 results

  Rule: Time ranges can be combined with text search

    Example: Filter search results by time
      Given I search for "message_type:ORU^R01"
      When I filter by time range "2023-11-01T00:00:00Z" to "2023-11-30T23:59:59Z"
      Then I should get 1 result
      And the result should have ID "MSG002"

    Example: Recent messages with specific content
      Given I search for "ADT"
      When I filter by time range "2023-12-01T00:00:00Z" to "2023-12-31T23:59:59Z"
      Then I should get 1 result
      And the result should have ID "MSG005"

  Rule: Time ranges handle edge cases

    Example: Start equals end time
      When I query messages from "2023-11-15T14:30:00Z" to "2023-11-15T14:30:00Z"
      Then I should get 1 result
      And the result should have timestamp "2023-11-15T14:30:00Z"

    Example: Inclusive range boundaries
      When I query messages from "2023-11-01T10:00:00Z" to "2023-11-30T08:00:00Z"
      Then I should get 3 results
      And the results should include "MSG001" and "MSG003"

    Example: Invalid time range (start after end)
      When I query messages from "2023-12-31T00:00:00Z" to "2023-01-01T00:00:00Z"
      Then the query should fail with error "Invalid time range"
