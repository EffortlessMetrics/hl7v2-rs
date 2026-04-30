Feature: HL7 v2 Escape Sequence Handling
  As an HL7 message processor
  I want to escape and unescape delimiter characters in HL7 text
  So that delimiter characters can be safely embedded in field values

  # =========================================================================
  # Escaping individual delimiters
  # =========================================================================

  Scenario: Escape field separator
    Given the text "before|after"
    And default delimiters
    When I escape the text
    Then the result should be "before\F\after"

  Scenario: Escape component separator
    Given the text "before^after"
    And default delimiters
    When I escape the text
    Then the result should be "before\S\after"

  Scenario: Escape repetition separator
    Given the text "before~after"
    And default delimiters
    When I escape the text
    Then the result should be "before\R\after"

  Scenario: Escape the escape character
    Given the text "before\after"
    And default delimiters
    When I escape the text
    Then the result should be "before\E\after"

  Scenario: Escape subcomponent separator
    Given the text "before&after"
    And default delimiters
    When I escape the text
    Then the result should be "before\T\after"

  # =========================================================================
  # Unescaping individual sequences
  # =========================================================================

  Scenario: Unescape field separator sequence
    Given the text "before\F\after"
    And default delimiters
    When I unescape the text
    Then the result should be "before|after"

  Scenario: Unescape component separator sequence
    Given the text "before\S\after"
    And default delimiters
    When I unescape the text
    Then the result should be "before^after"

  Scenario: Unescape repetition separator sequence
    Given the text "before\R\after"
    And default delimiters
    When I unescape the text
    Then the result should be "before~after"

  Scenario: Unescape the escape character sequence
    Given the text "before\E\after"
    And default delimiters
    When I unescape the text
    Then the result should be "before\after"

  Scenario: Unescape subcomponent separator sequence
    Given the text "before\T\after"
    And default delimiters
    When I unescape the text
    Then the result should be "before&after"

  # =========================================================================
  # Roundtrip: escape then unescape
  # =========================================================================

  Scenario: Roundtrip field separator
    Given the text "val|ue"
    And default delimiters
    When I escape then unescape the text
    Then the result should be "val|ue"

  Scenario: Roundtrip component separator
    Given the text "val^ue"
    And default delimiters
    When I escape then unescape the text
    Then the result should be "val^ue"

  Scenario: Roundtrip repetition separator
    Given the text "val~ue"
    And default delimiters
    When I escape then unescape the text
    Then the result should be "val~ue"

  Scenario: Roundtrip escape character
    Given the text "val\ue"
    And default delimiters
    When I escape then unescape the text
    Then the result should be "val\ue"

  Scenario: Roundtrip subcomponent separator
    Given the text "val&ue"
    And default delimiters
    When I escape then unescape the text
    Then the result should be "val&ue"

  # =========================================================================
  # Passthrough for plain text
  # =========================================================================

  Scenario: Text with no delimiters passes through escape unchanged
    Given the text "Hello World 123"
    And default delimiters
    When I escape the text
    Then the result should be "Hello World 123"

  Scenario: Text with no escape sequences passes through unescape unchanged
    Given the text "Hello World 123"
    And default delimiters
    When I unescape the text
    Then the result should be "Hello World 123"

  # =========================================================================
  # Custom delimiters
  # =========================================================================

  Scenario: Escape with custom delimiters
    Given the text "a#b$c*d@e!f"
    And custom delimiters "#$*@!"
    When I escape the text
    Then the result should be "a@F@b@S@c@R@d@E@e@T@f"

  Scenario: Unescape with custom delimiters
    Given the text "a@F@b@S@c@R@d@E@e@T@f"
    And custom delimiters "#$*@!"
    When I unescape the text
    Then the result should be "a#b$c*d@e!f"

  Scenario: Roundtrip with custom delimiters
    Given the text "x#y$z"
    And custom delimiters "#$*@!"
    When I escape then unescape the text
    Then the result should be "x#y$z"

  # =========================================================================
  # needs_escaping
  # =========================================================================

  Scenario: needs_escaping returns true for field separator
    Given the text "has|pipe"
    And default delimiters
    Then needs_escaping should return true

  Scenario: needs_escaping returns true for component separator
    Given the text "has^caret"
    And default delimiters
    Then needs_escaping should return true

  Scenario: needs_escaping returns true for repetition separator
    Given the text "has~tilde"
    And default delimiters
    Then needs_escaping should return true

  Scenario: needs_escaping returns true for escape character
    Given the text "has\backslash"
    And default delimiters
    Then needs_escaping should return true

  Scenario: needs_escaping returns true for subcomponent separator
    Given the text "has&ampersand"
    And default delimiters
    Then needs_escaping should return true

  Scenario: needs_escaping returns false for plain text
    Given the text "plain text"
    And default delimiters
    Then needs_escaping should return false

  # =========================================================================
  # needs_unescaping
  # =========================================================================

  Scenario: needs_unescaping returns true when escape sequences present
    Given the text "has\F\sequence"
    And default delimiters
    Then needs_unescaping should return true

  Scenario: needs_unescaping returns false for plain text
    Given the text "plain text"
    And default delimiters
    Then needs_unescaping should return false

  # =========================================================================
  # Empty string handling
  # =========================================================================

  Scenario: Escape empty string
    Given the text ""
    And default delimiters
    When I escape the text
    Then the result should be ""

  Scenario: Unescape empty string
    Given the text ""
    And default delimiters
    When I unescape the text
    Then the result should be ""

  Scenario: needs_escaping on empty string returns false
    Given the text ""
    And default delimiters
    Then needs_escaping should return false

  Scenario: needs_unescaping on empty string returns false
    Given the text ""
    And default delimiters
    Then needs_unescaping should return false

  # =========================================================================
  # Multiple delimiters in one string
  # =========================================================================

  Scenario: Escape multiple different delimiters in one string
    Given the text "a|b^c~d\e&f"
    And default delimiters
    When I escape the text
    Then the result should be "a\F\b\S\c\R\d\E\e\T\f"

  Scenario: Unescape multiple different sequences in one string
    Given the text "a\F\b\S\c\R\d\E\e\T\f"
    And default delimiters
    When I unescape the text
    Then the result should be "a|b^c~d\e&f"

  Scenario: Roundtrip multiple delimiters in one string
    Given the text "a|b^c~d\e&f"
    And default delimiters
    When I escape then unescape the text
    Then the result should be "a|b^c~d\e&f"
