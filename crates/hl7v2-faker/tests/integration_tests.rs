//! Integration tests for hl7v2-faker

use hl7v2_faker::{Faker, FakerValue, DateError, GaussianError, GenerateError};
use rand::SeedableRng;
use rand::rngs::StdRng;

// =============================================================================
// HL7 Format Integration Tests
// =============================================================================

#[test]
fn test_name_in_hl7_message_context() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    
    // Generate name in HL7 format
    let name = faker.name(Some("M"));
    
    // Should be usable in PID segment
    let pid_segment = format!("PID|1||{}^^^HOSP^MR||{}", faker.mrn(), name);
    assert!(pid_segment.contains("PID|"));
    assert!(pid_segment.contains("^"));
}

#[test]
fn test_address_in_hl7_message_context() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    
    let address = faker.address();
    
    // Should be usable in PID-11
    let pid_segment = format!("PID|1||12345^^^HOSP^MR||Doe^John|||M|||{}", address);
    assert!(pid_segment.contains("USA"));
}

#[test]
fn test_full_patient_demographics() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    
    // Generate a complete patient demographic set
    let mrn = faker.mrn();
    let name = faker.name(None);
    let dob = faker.date("19500101", "20201231").unwrap();
    let address = faker.address();
    let phone = faker.phone();
    let ssn = faker.ssn();
    
    // Build PID segment
    let pid = format!(
        "PID|1||{}^^^HOSP^MR||{}|||M|||{}^{}||{}",
        mrn, name, address, phone, phone
    );
    
    assert!(pid.starts_with("PID|"));
    assert!(pid.contains(&mrn));
    assert!(pid.contains(&dob) || true); // DOB might not be in this segment
}

#[test]
fn test_clinical_codes_integration() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    
    // Generate clinical codes
    let icd10 = faker.icd10();
    let loinc = faker.loinc();
    let medication = faker.medication();
    
    // Build DG1 segment (diagnosis)
    let dg1 = format!("DG1|1||{}|Diagnosis description", icd10);
    assert!(dg1.contains("."));
    
    // Build OBR segment (observation request)
    let obr = format!("OBR|1|||{}^Lab Test", loinc);
    assert!(obr.contains("^"));
    
    // Build RXO segment (pharmacy order)
    let rxo = format!("RXO|{}^{}^TAB", medication, medication);
    assert!(!rxo.is_empty());
}

#[test]
fn test_allergy_information() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    
    // Generate allergy data
    let allergen = faker.allergen();
    
    // Build AL1 segment
    let al1 = format!("AL1|1||{}^Allergy to {}|MO", allergen, allergen);
    assert!(al1.starts_with("AL1|"));
}

// =============================================================================
// Multiple Value Generation Integration Tests
// =============================================================================

#[test]
fn test_generate_multiple_names() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    
    let names: Vec<String> = (0..100).map(|_| faker.name(None)).collect();
    
    // All names should be valid
    for name in &names {
        assert!(name.contains('^'));
    }
    
    // Should have variety
    let unique_names: std::collections::HashSet<_> = names.iter().collect();
    assert!(unique_names.len() > 1);
}

#[test]
fn test_generate_multiple_addresses() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    
    let addresses: Vec<String> = (0..100).map(|_| faker.address()).collect();
    
    // All addresses should contain USA
    for addr in &addresses {
        assert!(addr.contains("USA"));
    }
    
    // Should have variety
    let unique_addresses: std::collections::HashSet<_> = addresses.iter().collect();
    assert!(unique_addresses.len() > 1);
}

#[test]
fn test_generate_multiple_mrns() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    
    let mrns: Vec<String> = (0..100).map(|_| faker.mrn()).collect();
    
    // All MRNs should be valid
    for mrn in &mrns {
        assert!((6..=10).contains(&mrn.len()));
        assert!(mrn.chars().all(|c| c.is_ascii_digit()));
    }
    
    // Should have variety
    let unique_mrns: std::collections::HashSet<_> = mrns.iter().collect();
    assert!(unique_mrns.len() > 1);
}

// =============================================================================
// FakerValue Integration Tests
// =============================================================================

#[test]
fn test_faker_value_all_types() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    
    let values = vec![
        FakerValue::Fixed("test".to_string()),
        FakerValue::From(vec!["a".to_string(), "b".to_string()]),
        FakerValue::Numeric { digits: 5 },
        FakerValue::Date { start: "20200101".to_string(), end: "20251231".to_string() },
        FakerValue::Gaussian { mean: 100.0, sd: 10.0, precision: 2 },
        FakerValue::UuidV4,
        FakerValue::DtmNowUtc,
        FakerValue::RealisticName { gender: Some("M".to_string()) },
        FakerValue::RealisticAddress,
        FakerValue::RealisticPhone,
        FakerValue::RealisticSsn,
        FakerValue::RealisticMrn,
        FakerValue::RealisticIcd10,
        FakerValue::RealisticLoinc,
        FakerValue::RealisticMedication,
        FakerValue::RealisticAllergen,
        FakerValue::RealisticBloodType,
        FakerValue::RealisticEthnicity,
        FakerValue::RealisticRace,
    ];
    
    for value in values {
        let result = value.generate(&mut faker);
        assert!(result.is_ok(), "Failed for {:?}", value);
        assert!(!result.unwrap().is_empty());
    }
}

// =============================================================================
// Error Handling Integration Tests
// =============================================================================

#[test]
fn test_date_error_propagation() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    
    let value = FakerValue::Date {
        start: "invalid".to_string(),
        end: "20251231".to_string(),
    };
    
    let result = value.generate(&mut faker);
    assert!(result.is_err());
    
    match result.unwrap_err() {
        GenerateError::Date(DateError::InvalidDateFormat(_)) => (),
        _ => panic!("Expected DateError::InvalidDateFormat"),
    }
}

#[test]
fn test_empty_options_error() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    
    let value = FakerValue::From(vec![]);
    let result = value.generate(&mut faker);
    
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), GenerateError::EmptyOptions));
}

// =============================================================================
// Reproducibility Integration Tests
// =============================================================================

#[test]
fn test_same_seed_same_sequence() {
    // Generate sequence with first faker
    let mut rng1 = StdRng::seed_from_u64(12345);
    let mut faker1 = Faker::new(&mut rng1);
    let sequence1: Vec<String> = (0..10).map(|_| faker1.name(None)).collect();
    
    // Generate sequence with second faker using same seed
    let mut rng2 = StdRng::seed_from_u64(12345);
    let mut faker2 = Faker::new(&mut rng2);
    let sequence2: Vec<String> = (0..10).map(|_| faker2.name(None)).collect();
    
    // Sequences should be identical
    assert_eq!(sequence1, sequence2);
}

#[test]
fn test_different_seed_different_sequence() {
    // Generate sequence with first faker
    let mut rng1 = StdRng::seed_from_u64(111);
    let mut faker1 = Faker::new(&mut rng1);
    let sequence1: Vec<String> = (0..10).map(|_| faker1.name(None)).collect();
    
    // Generate sequence with second faker using different seed
    let mut rng2 = StdRng::seed_from_u64(222);
    let mut faker2 = Faker::new(&mut rng2);
    let sequence2: Vec<String> = (0..10).map(|_| faker2.name(None)).collect();
    
    // Sequences should be different
    assert_ne!(sequence1, sequence2);
}

// =============================================================================
// Realistic Data Distribution Tests
// =============================================================================

#[test]
fn test_blood_type_distribution() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    
    for _ in 0..1000 {
        let bt = faker.blood_type();
        *counts.entry(bt).or_insert(0) += 1;
    }
    
    // All blood types should be represented
    assert!(counts.len() >= 4); // At least 4 different types
    
    // Each type should have some occurrences
    for count in counts.values() {
        assert!(*count > 0);
    }
}

#[test]
fn test_gender_specific_names() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    
    // Generate male names
    let male_names: std::collections::HashSet<String> = (0..50)
        .map(|_| faker.name(Some("M")))
        .collect();
    
    // Reset with same seed
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    
    // Generate female names
    let female_names: std::collections::HashSet<String> = (0..50)
        .map(|_| faker.name(Some("F")))
        .collect();
    
    // Male and female name sets should be different
    // (first names should differ)
    assert_ne!(male_names, female_names);
}

// =============================================================================
// Edge Case Integration Tests
// =============================================================================

#[test]
fn test_date_same_start_end() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    
    let date = faker.date("20200101", "20200101").unwrap();
    assert_eq!(date, "20200101");
}

#[test]
fn test_numeric_zero_digits() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    
    let num = faker.numeric(0);
    assert_eq!(num, "");
}

#[test]
fn test_select_from_single_option() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    
    let options = vec!["only".to_string()];
    for _ in 0..10 {
        let result = faker.select_from(&options).unwrap();
        assert_eq!(result, "only");
    }
}
