//! Integration tests for hl7v2-escape crate
//!
//! These tests verify escape/unescape works correctly with real-world
//! HL7 message scenarios.

use hl7v2_escape::{escape_text, unescape_text, needs_escaping, needs_unescaping};
use hl7v2_model::Delims;

// ============================================================================
// Real-World HL7 Field Value Tests
// ============================================================================

#[test]
fn test_escape_patient_name_with_pipe() {
    let delims = Delims::default();
    // Patient name containing a pipe character
    let name = "Smith|Jones";
    let escaped = escape_text(name, &delims);
    assert_eq!(escaped, "Smith\\F\\Jones");
    
    let unescaped = unescape_text(&escaped, &delims).unwrap();
    assert_eq!(unescaped, name);
}

#[test]
fn test_escape_address_with_special_chars() {
    let delims = Delims::default();
    // Address with multiple special characters
    let address = "123 Main St^Apt 4B~Unit 2";
    let escaped = escape_text(address, &delims);
    assert_eq!(escaped, "123 Main St\\S\\Apt 4B\\R\\Unit 2");
    
    let unescaped = unescape_text(&escaped, &delims).unwrap();
    assert_eq!(unescaped, address);
}

#[test]
fn test_escape_diagnosis_code() {
    let delims = Delims::default();
    // Diagnosis code with subcomponents
    let diagnosis = "A01.1& influenza";
    let escaped = escape_text(diagnosis, &delims);
    assert_eq!(escaped, "A01.1\\T\\ influenza");
    
    let unescaped = unescape_text(&escaped, &delims).unwrap();
    assert_eq!(unescaped, diagnosis);
}

#[test]
fn test_escape_file_path() {
    let delims = Delims::default();
    // Windows file path with backslashes
    let path = "C:\\Users\\Patient\\Documents";
    let escaped = escape_text(path, &delims);
    assert_eq!(escaped, "C:\\E\\Users\\E\\Patient\\E\\Documents");
    
    let unescaped = unescape_text(&escaped, &delims).unwrap();
    assert_eq!(unescaped, path);
}

// ============================================================================
// HL7 Message Field Tests
// ============================================================================

#[test]
fn test_escape_msh_field() {
    let delims = Delims::default();
    // MSH-2 encoding characters should not be escaped when already in proper format
    let encoding_chars = "^~\\&";
    // When this appears in field content, it should be escaped
    let escaped = escape_text(encoding_chars, &delims);
    assert_eq!(escaped, "\\S\\\\R\\\\E\\\\T\\");
}

#[test]
fn test_escape_obx_value() {
    let delims = Delims::default();
    // OBX-5 value with special characters
    let value = "Blood Pressure: 120|80 mmHg";
    let escaped = escape_text(value, &delims);
    assert_eq!(escaped, "Blood Pressure: 120\\F\\80 mmHg");
    
    let unescaped = unescape_text(&escaped, &delims).unwrap();
    assert_eq!(unescaped, value);
}

#[test]
fn test_escape_notes_field() {
    let delims = Delims::default();
    // Clinical notes with various special characters
    let notes = "Patient has history of: 1) Diabetes^Type 2, 2) Hypertension|Controlled";
    let escaped = escape_text(notes, &delims);
    assert_eq!(escaped, "Patient has history of: 1) Diabetes\\S\\Type 2, 2) Hypertension\\F\\Controlled");
    
    let unescaped = unescape_text(&escaped, &delims).unwrap();
    assert_eq!(unescaped, notes);
}

// ============================================================================
// Multi-Repetition Field Tests
// ============================================================================

#[test]
fn test_escape_multiple_repetitions() {
    let delims = Delims::default();
    // Field with repetitions containing special chars
    let field = "Value1^PartA~Value2^PartB";
    let escaped = escape_text(field, &delims);
    assert_eq!(escaped, "Value1\\S\\PartA\\R\\Value2\\S\\PartB");
    
    let unescaped = unescape_text(&escaped, &delims).unwrap();
    assert_eq!(unescaped, field);
}

#[test]
fn test_escape_phone_numbers() {
    let delims = Delims::default();
    // Phone numbers with different formats
    let phones = "(555)123-4567~(555)987-6543";
    let escaped = escape_text(phones, &delims);
    // Tilde (repetition separator) should be escaped
    assert_eq!(escaped, "(555)123-4567\\R\\(555)987-6543");
    
    let unescaped = unescape_text(&escaped, &delims).unwrap();
    assert_eq!(unescaped, phones);
}

// ============================================================================
// Complex Content Tests
// ============================================================================

#[test]
fn test_escape_clinical_data() {
    let delims = Delims::default();
    // Complex clinical data
    let data = "WBC: 7.5&10^3/uL|RBC: 5.0&10^6/uL";
    let escaped = escape_text(data, &delims);
    assert_eq!(escaped, "WBC: 7.5\\T\\10\\S\\3/uL\\F\\RBC: 5.0\\T\\10\\S\\6/uL");
    
    let unescaped = unescape_text(&escaped, &delims).unwrap();
    assert_eq!(unescaped, data);
}

#[test]
fn test_escape_medication_dosage() {
    let delims = Delims::default();
    // Medication dosage with special formatting
    let dosage = "Aspirin&325mg|Oral^Daily";
    let escaped = escape_text(dosage, &delims);
    assert_eq!(escaped, "Aspirin\\T\\325mg\\F\\Oral\\S\\Daily");
    
    let unescaped = unescape_text(&escaped, &delims).unwrap();
    assert_eq!(unescaped, dosage);
}

// ============================================================================
// Unicode and Internationalization Tests
// ============================================================================

#[test]
fn test_escape_international_characters() {
    let delims = Delims::default();
    // International characters should not be escaped
    let text = "Patient: Müller|François";
    let escaped = escape_text(text, &delims);
    assert_eq!(escaped, "Patient: Müller\\F\\François");
    
    let unescaped = unescape_text(&escaped, &delims).unwrap();
    assert_eq!(unescaped, text);
}

#[test]
fn test_escape_asian_characters() {
    let delims = Delims::default();
    // Asian characters with delimiters
    let text = "患者名|山田^太郎";
    let escaped = escape_text(text, &delims);
    assert_eq!(escaped, "患者名\\F\\山田\\S\\太郎");
    
    let unescaped = unescape_text(&escaped, &delims).unwrap();
    assert_eq!(unescaped, text);
}

#[test]
fn test_escape_arabic_characters() {
    let delims = Delims::default();
    // Arabic characters (RTL) with delimiters
    let text = "اسم|المريض";
    let escaped = escape_text(text, &delims);
    assert_eq!(escaped, "اسم\\F\\المريض");
    
    let unescaped = unescape_text(&escaped, &delims).unwrap();
    assert_eq!(unescaped, text);
}

// ============================================================================
// Edge Cases in Real-World Data
// ============================================================================

#[test]
fn test_escape_empty_parts() {
    let delims = Delims::default();
    // Field with empty parts
    let text = "||";
    let escaped = escape_text(text, &delims);
    assert_eq!(escaped, "\\F\\\\F\\");
    
    let unescaped = unescape_text(&escaped, &delims).unwrap();
    assert_eq!(unescaped, text);
}

#[test]
fn test_escape_only_special_chars() {
    let delims = Delims::default();
    // Only special characters
    let text = "|^~\\&";
    let escaped = escape_text(text, &delims);
    assert_eq!(escaped, "\\F\\\\S\\\\R\\\\E\\\\T\\");
    
    let unescaped = unescape_text(&escaped, &delims).unwrap();
    assert_eq!(unescaped, text);
}

#[test]
fn test_escape_long_text() {
    let delims = Delims::default();
    // Long clinical note
    let note = "This is a very long clinical note that contains various special characters like | (pipe), ^ (caret), ~ (tilde), \\ (backslash), and & (ampersand) scattered throughout the text.";
    let escaped = escape_text(note, &delims);
    
    // Verify all delimiters are escaped
    assert!(!escaped.contains('|'));
    assert!(!escaped.contains('^'));
    assert!(!escaped.contains('~'));
    assert!(!escaped.contains('&'));
    // Backslash should only appear in escape sequences
    
    let unescaped = unescape_text(&escaped, &delims).unwrap();
    assert_eq!(unescaped, note);
}

// ============================================================================
// Needs Escaping/Unescaping Integration Tests
// ============================================================================

#[test]
fn test_needs_escaping_real_fields() {
    let delims = Delims::default();
    
    // Fields that need escaping
    assert!(needs_escaping("Smith|Jones", &delims));
    assert!(needs_escaping("Value^Component", &delims));
    assert!(needs_escaping("Repeat1~Repeat2", &delims));
    assert!(needs_escaping("Path\\File", &delims));
    assert!(needs_escaping("Part1&Part2", &delims));
    
    // Fields that don't need escaping
    assert!(!needs_escaping("Normal Text", &delims));
    assert!(!needs_escaping("123 Main Street", &delims));
    assert!(!needs_escaping("Patient Name", &delims));
}

#[test]
fn test_needs_unescaping_real_fields() {
    let delims = Delims::default();
    
    // Fields that need unescaping
    assert!(needs_unescaping("Smith\\F\\Jones", &delims));
    assert!(needs_unescaping("Value\\S\\Component", &delims));
    assert!(needs_unescaping("Repeat1\\R\\Repeat2", &delims));
    
    // Fields that don't need unescaping
    assert!(!needs_unescaping("Normal Text", &delims));
    assert!(!needs_unescaping("No Escape Sequences", &delims));
}

// ============================================================================
// Custom Delimiter Integration Tests
// ============================================================================

#[test]
fn test_custom_delimiters_real_message() {
    // Simulate a message using non-standard delimiters
    let delims = Delims {
        field: '#',
        comp: ':',
        rep: '*',
        esc: '@',
        sub: '%',
    };
    
    // Test that delimiters in content are properly escaped
    let field_value = "Value1#Value2:Component*Repeat%Subcomponent";
    let escaped = escape_text(field_value, &delims);
    // The escape sequence uses the escape char @ followed by the letter code and @
    assert_eq!(escaped, "Value1@F@Value2@S@Component@R@Repeat@T@Subcomponent");
    
    let unescaped = unescape_text(&escaped, &delims).unwrap();
    assert_eq!(unescaped, field_value);
}

// ============================================================================
// Roundtrip Verification Tests
// ============================================================================

#[test]
fn test_roundtrip_various_content() {
    let delims = Delims::default();
    
    let test_cases = vec![
        "Simple text",
        "Text|with|pipes",
        "Component^separated^values",
        "Repeat~separated~values",
        "Path\\with\\backslashes",
        "Sub&component&data",
        "All|delimiters^in~one\\field&here",
        "",
        "|",
        "||||",
        "^",
        "~",
        "\\",
        "&",
        "Mixed|content^with~all\\types&here",
    ];
    
    for original in test_cases {
        let escaped = escape_text(original, &delims);
        let unescaped = unescape_text(&escaped, &delims).unwrap();
        assert_eq!(unescaped, original, "Failed roundtrip for: {:?}", original);
    }
}
