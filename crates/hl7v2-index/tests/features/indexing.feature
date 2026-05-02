Feature: Message Indexing
  As a healthcare integration developer
  I want to index HL7 messages for fast retrieval
  So that I can search and retrieve messages efficiently

  Background:
    Given a Tantivy backend is initialized with default configuration
    And the index is empty

  Rule: Messages are indexed by their unique ID

    Example: Index a single ADT message
      Given a valid ADT^A01 message:
        """
        MSH|^~\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20231119120000||ADT^A01|MSG001|P|2.5
        PID|1||MRN12345||Doe^John||19800101|M
        """
      When I index the message with ID "MSG001"
      Then the index should contain 1 document
      And I should be able to retrieve "MSG001" by ID

    Example: Index an ORU message with observation data
      Given a valid ORU^R01 message:
        """
        MSH|^~\&|LabSystem|LabFacility|EHRSystem|EHRFacility|20231119121500||ORU^R01|MSG002|P|2.5
        PID|1||MRN67890||Smith^Jane||19850315|F
        OBR|1||LAB001|GLUCOSE|||20231119120000
        OBX|1|NM|GLUCOSE^Glucose Level|1|120|mg/dL|70-100|N|||F
        """
      When I index the message with ID "MSG002"
      Then the index should contain 1 document
      And the stored message should have message type "ORU^R01"

  Rule: Batch indexing improves throughput

    Example: Index multiple messages with batching
      Given 1000 unique HL7 messages of type ORU^R01
      When I index all messages in a batch
      Then the index should contain 1000 documents
      And the flush operation should complete successfully

    Example: Index mixed message types
      Given 500 ADT^A01 messages
      And 500 ORU^R01 messages
      When I index all messages in a batch
      Then the index should contain 1000 documents
      And I can retrieve messages by type "ADT^A01"
      And I can retrieve messages by type "ORU^R01"

  Rule: Duplicate IDs are handled gracefully

    Example: Multiple messages with same ID are all stored
      Given a message with ID "MSG001" is already indexed
      When I attempt to index another message with ID "MSG001"
      Then the operation should succeed
      And the index should contain 2 documents with ID "MSG001"
      # Note: Deduplication is consumer responsibility

    Example: Update existing message via remove and add
      Given a message with ID "MSG001" and content "Original" is indexed
      When I remove message "MSG001"
      And I index a message with ID "MSG001" and content "Updated"
      Then I should be able to retrieve "MSG001" by ID
      And the retrieved message should have content "Updated"

  Rule: Index persistence survives restarts

    Example: Reopen existing index
      Given an index exists at path "/tmp/hl7_index"
      And the index contains 100 messages
      When I reopen the index at "/tmp/hl7_index"
      Then the index should contain 100 documents
      And I should be able to retrieve all 100 messages by ID

    Example: Index survives process restart
      Given an index contains 50 messages
      When the backend is closed and reopened
      Then all 50 messages should still be retrievable
