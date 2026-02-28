//! Sample HL7 messages and test data fixtures.
//!
//! This module provides access to sample HL7 v2 messages for testing purposes.
//! Messages are organized by type (valid samples, edge cases, and invalid messages).
//!
//! # Example
//!
//! ```rust,ignore
//! use hl7v2_test_utils::fixtures::SampleMessages;
//!
//! // Get a standard ADT^A01 message
//! let adt_a01 = SampleMessages::adt_a01();
//!
//! // Get an edge case message
//! let special = SampleMessages::edge_case("special_chars").unwrap();
//!
//! // Get an invalid message for error testing
//! let malformed = SampleMessages::invalid("malformed").unwrap();
//! ```

/// Sample HL7 messages for testing.
///
/// Provides access to various sample messages organized by category:
/// - Standard valid messages (ADT^A01, ADT^A04, ORU^R01)
/// - Edge case messages (escape sequences, custom delimiters, etc.)
/// - Invalid messages (malformed, truncated, etc.)
pub struct SampleMessages;

impl SampleMessages {
    /// Returns a sample ADT^A01 (Admit/Visit Notification) message.
    ///
    /// This is a commonly used message type for patient admissions.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let msg = SampleMessages::adt_a01();
    /// assert!(msg.starts_with("MSH|"));
    /// ```
    pub fn adt_a01() -> &'static str {
        concat!(
            "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|",
            "20250128152312||ADT^A01^ADT_A01|ABC123|P|2.5.1\r",
            "EVN|A01|20250128152312|||\r",
            "PID|1||123456^^^HOSP^MR||Doe^John^A||19800101|M|||C|\r",
            "PV1|1|I|ICU^101^01||||DOC123^Smith^Jane||||||||V123456\r"
        )
    }

    /// Returns a sample ADT^A04 (Register Patient) message.
    ///
    /// This message type is used for patient registration.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let msg = SampleMessages::adt_a04();
    /// assert!(msg.contains("ADT^A04"));
    /// ```
    pub fn adt_a04() -> &'static str {
        concat!(
            "MSH|^~\\&|RegSys|Hospital|ADT|Hospital|",
            "20250128140000||ADT^A04|MSG002|P|2.5\r",
            "PID|1||MRN456^^^Hospital^MR||Smith^Jane^M||19900215|F\r"
        )
    }

    /// Returns a sample ORU^R01 (Lab Results) message.
    ///
    /// This message type is used for transmitting laboratory results.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let msg = SampleMessages::oru_r01();
    /// assert!(msg.contains("ORU^R01"));
    /// ```
    pub fn oru_r01() -> &'static str {
        concat!(
            "MSH|^~\\&|LabSys|Lab|LIS|Hospital|",
            "20250128150000||ORU^R01|MSG003|P|2.5\r",
            "PID|1||MRN789^^^Lab^MR||Patient^Test||19850610|M\r",
            "OBR|1|ORD123|FIL456|CBC^Complete Blood Count|||20250128120000\r",
            "OBX|1|NM|WBC^White Blood Count||7.5|10^9/L|4.0-11.0|N|||F\r"
        )
    }

    /// Returns an edge case message by name.
    ///
    /// Available edge cases:
    /// - `empty_fields` - Message with empty field values
    /// - `max_lengths` - Message with maximum length field values
    /// - `special_chars` - Message with special characters and escape sequences
    /// - `custom_delims` - Message with non-standard delimiters
    /// - `with_repetitions` - Message with field repetitions
    /// - `fully_populated` - Message with all optional components
    ///
    /// # Errors
    ///
    /// Returns `None` if the named edge case doesn't exist.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let msg = SampleMessages::edge_case("special_chars").unwrap();
    /// ```
    pub fn edge_case(name: &str) -> Option<&'static str> {
        match name {
            "empty_fields" => Some(EDGE_CASE_EMPTY_FIELDS),
            "max_lengths" => Some(EDGE_CASE_MAX_LENGTHS),
            "special_chars" => Some(EDGE_CASE_SPECIAL_CHARS),
            "custom_delims" => Some(EDGE_CASE_CUSTOM_DELIMS),
            "with_repetitions" => Some(EDGE_CASE_REPETITIONS),
            "fully_populated" => Some(EDGE_CASE_FULLY_POPULATED),
            _ => None,
        }
    }

    /// Returns an invalid message by name.
    ///
    /// Available invalid messages:
    /// - `malformed` - Message with structural errors
    /// - `truncated` - Message that is incomplete
    /// - `no_msh` - Message missing the required MSH segment
    /// - `bad_encoding` - Message with invalid encoding characters
    /// - `bad_terminator` - Message with invalid segment terminators
    ///
    /// # Errors
    ///
    /// Returns `None` if the named invalid message doesn't exist.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let msg = SampleMessages::invalid("malformed").unwrap();
    /// ```
    pub fn invalid(name: &str) -> Option<&'static str> {
        match name {
            "malformed" => Some(INVALID_MALFORMED),
            "truncated" => Some(INVALID_TRUNCATED),
            "no_msh" => Some(INVALID_NO_MSH),
            "bad_encoding" => Some(INVALID_BAD_ENCODING),
            "bad_terminator" => Some(INVALID_BAD_TERMINATOR),
            _ => None,
        }
    }

    /// Returns all valid sample messages as an iterator.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// for (name, msg) in SampleMessages::all_valid() {
    ///     println!("Testing message: {}", name);
    ///     assert!(msg.starts_with("MSH|"));
    /// }
    /// ```
    pub fn all_valid() -> impl Iterator<Item = (&'static str, &'static str)> {
        vec![
            ("ADT_A01", Self::adt_a01()),
            ("ADT_A04", Self::adt_a04()),
            ("ORU_R01", Self::oru_r01()),
        ]
        .into_iter()
    }

    /// Returns all edge case messages as an iterator.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// for (name, msg) in SampleMessages::all_edge_cases() {
    ///     println!("Testing edge case: {}", name);
    /// }
    /// ```
    pub fn all_edge_cases() -> impl Iterator<Item = (&'static str, &'static str)> {
        vec![
            ("empty_fields", EDGE_CASE_EMPTY_FIELDS),
            ("max_lengths", EDGE_CASE_MAX_LENGTHS),
            ("special_chars", EDGE_CASE_SPECIAL_CHARS),
            ("custom_delims", EDGE_CASE_CUSTOM_DELIMS),
            ("with_repetitions", EDGE_CASE_REPETITIONS),
            ("fully_populated", EDGE_CASE_FULLY_POPULATED),
        ]
        .into_iter()
    }

    /// Returns all invalid messages as an iterator.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// for (name, msg) in SampleMessages::all_invalid() {
    ///     println!("Testing invalid message: {}", name);
    /// }
    /// ```
    pub fn all_invalid() -> impl Iterator<Item = (&'static str, &'static str)> {
        vec![
            ("malformed", INVALID_MALFORMED),
            ("truncated", INVALID_TRUNCATED),
            ("no_msh", INVALID_NO_MSH),
            ("bad_encoding", INVALID_BAD_ENCODING),
            ("bad_terminator", INVALID_BAD_TERMINATOR),
        ]
        .into_iter()
    }
}

// Edge case message constants
const EDGE_CASE_EMPTY_FIELDS: &str = concat!(
    "MSH|^~\\&|SendingApp|SendingFac|||||ADT^A01|1|P|2.5\r",
    "PID|1|||||||||||\r",
    "PV1|1||||||||||||||\r"
);

const EDGE_CASE_MAX_LENGTHS: &str = concat!(
    "MSH|^~\\&|SendingAppWithVeryLongName|SendingFacilityWithVeryLongName|",
    "ReceivingAppWithVeryLongName|ReceivingFacilityWithVeryLongName|",
    "20250128152312||ADT^A01|ABC12345678901234567890|P|2.5.1\r",
    "PID|1||1234567890123456789012345678901234567890^^^HOSPITAL^MR||",
    "VeryLongLastName^VeryLongFirstName^VeryLongMiddleName||19800101|M\r"
);

const EDGE_CASE_SPECIAL_CHARS: &str = concat!(
    "MSH|^~\\&|App|Fac|||20250128120000||ADT^A01|1|P|2.5\r",
    "PID|1||123||Test\\F\\Value\\S\\Special\\R\\Chars\\E\\Escape\r"
);

const EDGE_CASE_CUSTOM_DELIMS: &str = concat!(
    "MSH#$*@!App#Fac#Rec#RecFac#20250128120000##ADT$A01#1#P#2.5\r",
    "PID#1##123##Name$First\r"
);

const EDGE_CASE_REPETITIONS: &str = concat!(
    "MSH|^~\\&|App|Fac|||20250128120000||ADT^A01|1|P|2.5\r",
    "PID|1||123||Doe^John~Smith^Jane~Brown^Bob\r"
);

const EDGE_CASE_FULLY_POPULATED: &str = concat!(
    "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|",
    "20250128152312+0000||ADT^A01^ADT_A01|ABC123|P|2.5.1|||AL|NE|ASCII\r",
    "EVN|A01|20250128152312+0000|20250128160000||DOC123^Smith^Jane^^^^MD^^^NPI^12345\r",
    "PID|1||123456^^^HOSP^MR||Doe^John^Adam^III^Sr.||19800101|M||C|",
    "123 Main St^Apt 4B^Anytown^ST^12345^USA||(555)555-1212|(555)555-1213||E|S|||123456789|\r"
);

// Invalid message constants
const INVALID_MALFORMED: &str = concat!(
    "MSH|^~\\&|App|Fac|||20250128120000||ADT^A01|1|P|2.5\r",
    "PID||||\r",
    "INVALID SEGMENT\r",
    "PV1|1|I\r"
);

const INVALID_TRUNCATED: &str = "MSH|^~\\&|App|Fac";

const INVALID_NO_MSH: &str = "PID|1||123||Doe^John\r";

const INVALID_BAD_ENCODING: &str = "MSH|SendingApp\r";

const INVALID_BAD_TERMINATOR: &str = "MSH|^~\\&|App|Fac\nPID|1||123";

/// Sample segment fixtures for testing individual segments.
pub struct SampleSegments;

impl SampleSegments {
    /// Returns a sample MSH (Message Header) segment.
    pub fn msh() -> &'static str {
        "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01|ABC123|P|2.5.1"
    }

    /// Returns a sample PID (Patient Identification) segment.
    pub fn pid() -> &'static str {
        "PID|1||123456^^^HOSP^MR||Doe^John^A||19800101|M|||C|"
    }

    /// Returns a sample PV1 (Patient Visit) segment.
    pub fn pv1() -> &'static str {
        "PV1|1|I|ICU^101^01||||DOC123^Smith^Jane||||||||V123456"
    }

    /// Returns a sample OBX (Observation Result) segment.
    pub fn obx() -> &'static str {
        "OBX|1|NM|WBC^White Blood Count||7.5|10^9/L|4.0-11.0|N|||F"
    }

    /// Returns a sample OBR (Observation Request) segment.
    pub fn obr() -> &'static str {
        "OBR|1|ORD123|FIL456|CBC^Complete Blood Count|||20250128120000"
    }

    /// Returns a sample EVN (Event Type) segment.
    pub fn evn() -> &'static str {
        "EVN|A01|20250128152312|||"
    }

    /// Returns a sample NK1 (Next of Kin) segment.
    pub fn nk1() -> &'static str {
        "NK1|1|Doe^Jane|SPO|123 Main St^Anytown^ST^12345||(555)555-1212"
    }

    /// Returns a sample AL1 (Patient Allergy) segment.
    pub fn al1() -> &'static str {
        "AL1|1|DA|Penicillin||Causes rash|20200101"
    }

    /// Returns a sample DG1 (Diagnosis) segment.
    pub fn dg1() -> &'static str {
        "DG1|1|I10|J18.9^Pneumonia||20250128"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adt_a01_is_valid() {
        let msg = SampleMessages::adt_a01();
        assert!(msg.starts_with("MSH|"));
        assert!(msg.contains("ADT^A01"));
    }

    #[test]
    fn test_adt_a04_is_valid() {
        let msg = SampleMessages::adt_a04();
        assert!(msg.starts_with("MSH|"));
        assert!(msg.contains("ADT^A04"));
    }

    #[test]
    fn test_oru_r01_is_valid() {
        let msg = SampleMessages::oru_r01();
        assert!(msg.starts_with("MSH|"));
        assert!(msg.contains("ORU^R01"));
    }

    #[test]
    fn test_edge_case_empty_fields() {
        let msg = SampleMessages::edge_case("empty_fields").unwrap();
        assert!(msg.contains("|||||"));
    }

    #[test]
    fn test_edge_case_special_chars() {
        let msg = SampleMessages::edge_case("special_chars").unwrap();
        assert!(msg.contains("\\F\\"));
        assert!(msg.contains("\\S\\"));
    }

    #[test]
    fn test_edge_case_not_found() {
        let result = SampleMessages::edge_case("nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_invalid_malformed() {
        let msg = SampleMessages::invalid("malformed").unwrap();
        assert!(msg.contains("INVALID SEGMENT"));
    }

    #[test]
    fn test_invalid_truncated() {
        let msg = SampleMessages::invalid("truncated").unwrap();
        assert!(!msg.ends_with("\r"));
    }

    #[test]
    fn test_invalid_not_found() {
        let result = SampleMessages::invalid("nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_all_valid_count() {
        let count = SampleMessages::all_valid().count();
        assert_eq!(count, 3);
    }

    #[test]
    fn test_all_edge_cases_count() {
        let count = SampleMessages::all_edge_cases().count();
        assert_eq!(count, 6);
    }

    #[test]
    fn test_all_invalid_count() {
        let count = SampleMessages::all_invalid().count();
        assert_eq!(count, 5);
    }

    #[test]
    fn test_sample_segments() {
        assert!(SampleSegments::msh().starts_with("MSH|"));
        assert!(SampleSegments::pid().starts_with("PID|"));
        assert!(SampleSegments::pv1().starts_with("PV1|"));
        assert!(SampleSegments::obx().starts_with("OBX|"));
        assert!(SampleSegments::obr().starts_with("OBR|"));
        assert!(SampleSegments::evn().starts_with("EVN|"));
        assert!(SampleSegments::nk1().starts_with("NK1|"));
        assert!(SampleSegments::al1().starts_with("AL1|"));
        assert!(SampleSegments::dg1().starts_with("DG1|"));
    }
}
