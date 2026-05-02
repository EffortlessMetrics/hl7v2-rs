Feature: Backend Performance
  As a performance engineer
  I want to compare Tantivy and SQLite backends
  So that I can select the appropriate backend for my workload

  Background:
    Given a corpus of 10000 HL7 messages
    And both Tantivy and SQLite backends are configured
    And the data directory is isolated for each backend

  Rule: Index throughput meets performance targets

    Example: Tantivy indexes messages quickly
      When I index the corpus with Tantivy backend
      Then the operation should complete in under 2 seconds
      And Tantivy should index at least 5000 messages per second
      And the index should contain all 10000 messages

    Example: SQLite has acceptable index throughput
      When I index the corpus with SQLite backend
      Then the operation should complete in under 10 seconds
      And SQLite should index at least 1000 messages per second
      And the index should contain all 10000 messages

    Example: Batch indexing improves throughput
      Given individual message indexing averages 1ms per message
      When I index 1000 messages in batches of 100
      Then average time per message should be less than 0.5ms

  Rule: Search latency meets response time targets

    Example: Tantivy has low search latency
      Given the corpus is indexed in Tantivy
      When I execute 100 full-text searches
      Then the average latency should be under 10ms
      And the p99 latency should be under 50ms
      And no search should take longer than 100ms

    Example: SQLite has acceptable search latency
      Given the corpus is indexed in SQLite
      When I execute 100 full-text searches
      Then the average latency should be under 50ms
      And the p99 latency should be under 200ms

    Example: Cold start vs warm cache
      Given the corpus is indexed in Tantivy
      When I execute a search on first query
      Then the latency may be higher than 10ms
      When I execute the same search again
      Then the latency should be under 5ms

  Rule: Index scales with data volume

    Example: Linear scaling test
      Given I start with 1000 messages indexed
      When I add 1000 messages 10 times
      Then each addition should complete in under 5 seconds
      And search latency should remain under 20ms
      And the final index should contain 11000 messages

    Example: Large message handling
      Given messages with average size of 10KB
      When I index 1000 large messages
      Then the index size on disk should be under 20MB
      And search operations should complete successfully

  Rule: Memory usage remains bounded

    Example: Tantivy memory during indexing
      Given a memory budget of 200MB
      When I index 10000 messages
      Then peak memory usage should not exceed 200MB

    Example: Memory after indexing completes
      Given 10000 messages are indexed
      When indexing is complete and writer is flushed
      Then memory usage should drop below 50MB

  Rule: Concurrent access is handled safely

    Example: Multiple readers during indexing
      Given the corpus is being indexed
      When 5 concurrent searches are executed
      Then all searches should complete without error
      And no data corruption should occur

    Example: Writer thread safety
      Given 10 threads attempting to index messages
      When all threads index 100 messages each
      Then the final index should contain 1000 messages
      And no panics or deadlocks should occur
