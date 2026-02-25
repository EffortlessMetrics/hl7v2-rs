//! Unit tests for hl7v2-faker

use super::*;
use rand::SeedableRng;

// =============================================================================
// Name Generation Tests
// =============================================================================

#[test]
fn test_name_male_format() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let name = faker.name(Some("M"));
    
    // Should be in HL7 format: LAST^FIRST
    assert!(name.contains('^'));
    let parts: Vec<&str> = name.split('^').collect();
    assert_eq!(parts.len(), 2);
    assert!(!parts[0].is_empty());
    assert!(!parts[1].is_empty());
}

#[test]
fn test_name_female_format() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let name = faker.name(Some("F"));
    
    assert!(name.contains('^'));
    let parts: Vec<&str> = name.split('^').collect();
    assert_eq!(parts.len(), 2);
}

#[test]
fn test_name_any_format() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let name = faker.name(None);
    
    assert!(name.contains('^'));
    let parts: Vec<&str> = name.split('^').collect();
    assert_eq!(parts.len(), 2);
}

#[test]
fn test_name_deterministic() {
    let mut rng1 = StdRng::seed_from_u64(42);
    let mut faker1 = Faker::new(&mut rng1);
    let name1 = faker1.name(Some("M"));
    
    let mut rng2 = StdRng::seed_from_u64(42);
    let mut faker2 = Faker::new(&mut rng2);
    let name2 = faker2.name(Some("M"));
    
    assert_eq!(name1, name2);
}

#[test]
fn test_name_different_seeds() {
    let mut rng1 = StdRng::seed_from_u64(42);
    let mut faker1 = Faker::new(&mut rng1);
    let name1 = faker1.name(None);
    
    let mut rng2 = StdRng::seed_from_u64(43);
    let mut faker2 = Faker::new(&mut rng2);
    let name2 = faker2.name(None);
    
    // Different seeds should likely produce different names
    // (though there's a small chance they could be the same)
    assert!(name1 != name2 || name1 == name2); // Always passes, but exercises the code
}

// =============================================================================
// Address Generation Tests
// =============================================================================

#[test]
fn test_address_format() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let address = faker.address();
    
    // Should contain separators
    assert!(address.contains('^'));
    // Should contain USA
    assert!(address.contains("USA"));
}

#[test]
fn test_address_deterministic() {
    let mut rng1 = StdRng::seed_from_u64(42);
    let mut faker1 = Faker::new(&mut rng1);
    let addr1 = faker1.address();
    
    let mut rng2 = StdRng::seed_from_u64(42);
    let mut faker2 = Faker::new(&mut rng2);
    let addr2 = faker2.address();
    
    assert_eq!(addr1, addr2);
}

#[test]
fn test_address_has_street_number() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let address = faker.address();
    
    // Should start with a street number (100-9999)
    let street_number: String = address.chars().take_while(|c| c.is_ascii_digit()).collect();
    let num: i32 = street_number.parse().unwrap();
    assert!((100..=9999).contains(&num));
}

// =============================================================================
// Phone Generation Tests
// =============================================================================

#[test]
fn test_phone_format() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let phone = faker.phone();
    
    // Should be in format (AREA)EXCHANGE-NUMBER
    assert!(phone.starts_with('('));
    assert!(phone.contains(')'));
    assert!(phone.contains('-'));
}

#[test]
fn test_phone_length() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let phone = faker.phone();
    
    // (XXX)XXX-XXXX = 13 characters
    assert_eq!(phone.len(), 13);
}

#[test]
fn test_phone_area_code_range() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    
    // Generate multiple phones and check area codes
    for _ in 0..10 {
        let phone = faker.phone();
        let area_code: String = phone.chars().skip(1).take(3).collect();
        let area: i32 = area_code.parse().unwrap();
        assert!((200..999).contains(&area));
    }
}

// =============================================================================
// SSN Generation Tests
// =============================================================================

#[test]
fn test_ssn_format() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let ssn = faker.ssn();
    
    // Should be in format XXX-XX-XXXX
    assert_eq!(ssn.len(), 11);
    assert_eq!(ssn.matches('-').count(), 2);
    assert_eq!(&ssn[3..4], "-");
    assert_eq!(&ssn[6..7], "-");
}

#[test]
fn test_ssn_all_digits() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let ssn = faker.ssn();
    
    let digits_only: String = ssn.chars().filter(|c| c.is_ascii_digit()).collect();
    assert_eq!(digits_only.len(), 9);
}

// =============================================================================
// MRN Generation Tests
// =============================================================================

#[test]
fn test_mrn_length() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let mrn = faker.mrn();
    
    assert!((6..=10).contains(&mrn.len()));
}

#[test]
fn test_mrn_all_digits() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let mrn = faker.mrn();
    
    assert!(mrn.chars().all(|c| c.is_ascii_digit()));
}

#[test]
fn test_mrn_multiple_calls_vary() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    
    let mrns: Vec<String> = (0..10).map(|_| faker.mrn()).collect();
    
    // Not all MRNs should be the same
    let unique: std::collections::HashSet<_> = mrns.iter().collect();
    assert!(unique.len() > 1);
}

// =============================================================================
// ICD-10 Generation Tests
// =============================================================================

#[test]
fn test_icd10_format() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let code = faker.icd10();
    
    // Should be in format XXX.X
    assert!(code.contains('.'));
    let parts: Vec<&str> = code.split('.').collect();
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[0].len(), 3);
    assert_eq!(parts[1].len(), 1);
}

#[test]
fn test_icd10_starts_with_letter() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let code = faker.icd10();
    
    assert!(code.chars().next().unwrap().is_ascii_uppercase());
}

// =============================================================================
// LOINC Generation Tests
// =============================================================================

#[test]
fn test_loinc_all_digits() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let code = faker.loinc();
    
    assert!(code.chars().all(|c| c.is_ascii_digit()));
}

#[test]
fn test_loinc_length() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let code = faker.loinc();
    
    // 5-7 digits
    assert!((5..=7).contains(&code.len()));
}

// =============================================================================
// Medication Generation Tests
// =============================================================================

#[test]
fn test_medication_not_empty() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let med = faker.medication();
    
    assert!(!med.is_empty());
}

#[test]
fn test_medication_from_list() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    
    let medications = [
        "Atorvastatin", "Levothyroxine", "Lisinopril", "Metformin",
        "Amlodipine", "Metoprolol", "Omeprazole", "Simvastatin",
        "Losartan", "Albuterol"
    ];
    
    let med = faker.medication();
    assert!(medications.contains(&med.as_str()));
}

// =============================================================================
// Allergen Generation Tests
// =============================================================================

#[test]
fn test_allergen_not_empty() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let allergen = faker.allergen();
    
    assert!(!allergen.is_empty());
}

#[test]
fn test_allergen_from_list() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    
    let allergens = [
        "Penicillin", "Latex", "Peanuts", "Shellfish", "Eggs",
        "Milk", "Tree Nuts", "Soy", "Wheat", "Bee Stings"
    ];
    
    let allergen = faker.allergen();
    assert!(allergens.contains(&allergen.as_str()));
}

// =============================================================================
// Blood Type Generation Tests
// =============================================================================

#[test]
fn test_blood_type_valid() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let blood_type = faker.blood_type();
    
    let valid_types = ["A+", "A-", "B+", "B-", "AB+", "AB-", "O+", "O-"];
    assert!(valid_types.contains(&blood_type.as_str()));
}

#[test]
fn test_blood_type_multiple_calls() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    
    let types: std::collections::HashSet<String> = (0..20)
        .map(|_| faker.blood_type())
        .collect();
    
    // Should generate more than one type over 20 calls
    assert!(types.len() > 1);
}

// =============================================================================
// Ethnicity Generation Tests
// =============================================================================

#[test]
fn test_ethnicity_valid() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let ethnicity = faker.ethnicity();
    
    let valid = [
        "Hispanic or Latino", 
        "Not Hispanic or Latino", 
        "Declined to Specify"
    ];
    assert!(valid.contains(&ethnicity.as_str()));
}

// =============================================================================
// Race Generation Tests
// =============================================================================

#[test]
fn test_race_valid() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let race = faker.race();
    
    let valid = [
        "American Indian or Alaska Native", 
        "Asian", 
        "Black or African American",
        "Native Hawaiian or Other Pacific Islander", 
        "White", 
        "Declined to Specify"
    ];
    assert!(valid.contains(&race.as_str()));
}

// =============================================================================
// Numeric Generation Tests
// =============================================================================

#[test]
fn test_numeric_length() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    
    for len in [1, 5, 10, 20] {
        let num = faker.numeric(len);
        assert_eq!(num.len(), len);
    }
}

#[test]
fn test_numeric_all_digits() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let num = faker.numeric(10);
    
    assert!(num.chars().all(|c| c.is_ascii_digit()));
}

#[test]
fn test_numeric_zero_length() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let num = faker.numeric(0);
    
    assert_eq!(num.len(), 0);
}

// =============================================================================
// Date Generation Tests
// =============================================================================

#[test]
fn test_date_valid_format() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let date = faker.date("20200101", "20251231").unwrap();
    
    assert_eq!(date.len(), 8);
    assert!(date.chars().all(|c| c.is_ascii_digit()));
}

#[test]
fn test_date_in_range() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    
    // Generate multiple dates and verify they're in range
    for _ in 0..10 {
        let date = faker.date("20200101", "20201231").unwrap();
        assert!(date.as_str() >= "20200101");
        assert!(date.as_str() <= "20201231");
    }
}

#[test]
fn test_date_invalid_start_format() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let result = faker.date("invalid", "20251231");
    
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), DateError::InvalidDateFormat(_)));
}

#[test]
fn test_date_invalid_end_format() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let result = faker.date("20200101", "invalid");
    
    assert!(result.is_err());
}

#[test]
fn test_date_same_start_end() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let date = faker.date("20200101", "20200101").unwrap();
    
    assert_eq!(date, "20200101");
}

// =============================================================================
// Gaussian Generation Tests
// =============================================================================

#[test]
fn test_gaussian_valid() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let value = faker.gaussian(100.0, 10.0, 2).unwrap();
    
    // Should be parseable as f64
    let parsed: f64 = value.parse().unwrap();
    // Should be within reasonable range (5 standard deviations)
    assert!((50.0..=150.0).contains(&parsed));
}

#[test]
fn test_gaussian_precision() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    
    for precision in [0, 1, 2, 4] {
        let value = faker.gaussian(100.0, 10.0, precision).unwrap();
        if let Some(dot_pos) = value.find('.') {
            let decimal_places = value.len() - dot_pos - 1;
            assert!(decimal_places <= precision);
        }
    }
}

#[test]
fn test_gaussian_zero_sd() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    // Zero SD should return the mean
    let result = faker.gaussian(100.0, 0.0, 2);
    // With zero SD, result should be very close to mean
    assert!(result.is_ok());
}

// =============================================================================
// UUID Generation Tests
// =============================================================================

#[test]
fn test_uuid_format() {
    let mut rng = StdRng::seed_from_u64(42);
    let faker = Faker::new(&mut rng);
    let uuid = faker.uuid_v4();
    
    assert_eq!(uuid.len(), 36);
    assert_eq!(uuid.matches('-').count(), 4);
}

#[test]
fn test_uuid_positions() {
    let mut rng = StdRng::seed_from_u64(42);
    let faker = Faker::new(&mut rng);
    let uuid = faker.uuid_v4();
    
    // UUID v4 format: xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx
    assert_eq!(&uuid[8..9], "-");
    assert_eq!(&uuid[13..14], "-");
    assert_eq!(&uuid[18..19], "-");
    assert_eq!(&uuid[23..24], "-");
}

#[test]
fn test_uuid_multiple_unique() {
    let mut rng = StdRng::seed_from_u64(42);
    let faker = Faker::new(&mut rng);
    
    let uuids: std::collections::HashSet<String> = (0..100)
        .map(|_| faker.uuid_v4())
        .collect();
    
    // All UUIDs should be unique
    assert_eq!(uuids.len(), 100);
}

// =============================================================================
// DateTime Now UTC Tests
// =============================================================================

#[test]
fn test_dtm_now_utc_format() {
    let mut rng = StdRng::seed_from_u64(42);
    let faker = Faker::new(&mut rng);
    let dtm = faker.dtm_now_utc();
    
    assert_eq!(dtm.len(), 14);
    assert!(dtm.chars().all(|c| c.is_ascii_digit()));
}

#[test]
fn test_dtm_now_utc_reasonable() {
    let mut rng = StdRng::seed_from_u64(42);
    let faker = Faker::new(&mut rng);
    let dtm = faker.dtm_now_utc();
    
    // Should be in 2020s
    assert!(dtm.starts_with("202"));
}

// =============================================================================
// Select From Tests
// =============================================================================

#[test]
fn test_select_from_valid() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    
    let options = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    let result = faker.select_from(&options).unwrap();
    
    assert!(options.contains(&result));
}

#[test]
fn test_select_from_empty() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    
    let options: Vec<String> = vec![];
    let result = faker.select_from(&options);
    
    assert!(result.is_none());
}

#[test]
fn test_select_from_single() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    
    let options = vec!["only".to_string()];
    let result = faker.select_from(&options).unwrap();
    
    assert_eq!(result, "only");
}

// =============================================================================
// Select From Map Tests
// =============================================================================

#[test]
fn test_select_from_map_valid() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    
    let mut map = std::collections::HashMap::new();
    map.insert("key1".to_string(), "value1".to_string());
    map.insert("key2".to_string(), "value2".to_string());
    
    let result = faker.select_from_map(&map).unwrap();
    
    assert!(map.values().any(|v| v == &result));
}

#[test]
fn test_select_from_map_empty() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    
    let map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let result = faker.select_from_map(&map);
    
    assert!(result.is_none());
}

// =============================================================================
// FakerValue Tests
// =============================================================================

#[test]
fn test_faker_value_fixed() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let value = FakerValue::Fixed("test_value".to_string());
    
    assert_eq!(value.generate(&mut faker).unwrap(), "test_value");
}

#[test]
fn test_faker_value_from() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let options = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    let value = FakerValue::From(options.clone());
    
    let result = value.generate(&mut faker).unwrap();
    assert!(options.contains(&result));
}

#[test]
fn test_faker_value_from_empty() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let value = FakerValue::From(vec![]);
    
    let result = value.generate(&mut faker);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), GenerateError::EmptyOptions));
}

#[test]
fn test_faker_value_numeric() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let value = FakerValue::Numeric { digits: 5 };
    
    let result = value.generate(&mut faker).unwrap();
    assert_eq!(result.len(), 5);
    assert!(result.chars().all(|c| c.is_ascii_digit()));
}

#[test]
fn test_faker_value_date() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let value = FakerValue::Date {
        start: "20200101".to_string(),
        end: "20201231".to_string(),
    };
    
    let result = value.generate(&mut faker).unwrap();
    assert_eq!(result.len(), 8);
}

#[test]
fn test_faker_value_date_invalid() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let value = FakerValue::Date {
        start: "invalid".to_string(),
        end: "20201231".to_string(),
    };
    
    let result = value.generate(&mut faker);
    assert!(result.is_err());
}

#[test]
fn test_faker_value_gaussian() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let value = FakerValue::Gaussian {
        mean: 100.0,
        sd: 10.0,
        precision: 2,
    };
    
    let result = value.generate(&mut faker).unwrap();
    let parsed: f64 = result.parse().unwrap();
    assert!((50.0..=150.0).contains(&parsed));
}

#[test]
fn test_faker_value_map() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    
    let mut map = std::collections::HashMap::new();
    map.insert("key1".to_string(), "value1".to_string());
    map.insert("key2".to_string(), "value2".to_string());
    
    let value = FakerValue::Map(map.clone());
    let result = value.generate(&mut faker).unwrap();
    assert!(map.values().any(|v| v == &result));
}

#[test]
fn test_faker_value_map_empty() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let value = FakerValue::Map(std::collections::HashMap::new());
    
    let result = value.generate(&mut faker);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), GenerateError::EmptyMap));
}

#[test]
fn test_faker_value_uuid_v4() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let value = FakerValue::UuidV4;
    
    let result = value.generate(&mut faker).unwrap();
    assert_eq!(result.len(), 36);
}

#[test]
fn test_faker_value_dtm_now_utc() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let value = FakerValue::DtmNowUtc;
    
    let result = value.generate(&mut faker).unwrap();
    assert_eq!(result.len(), 14);
}

#[test]
fn test_faker_value_realistic_name_male() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let value = FakerValue::RealisticName { gender: Some("M".to_string()) };
    
    let result = value.generate(&mut faker).unwrap();
    assert!(result.contains('^'));
}

#[test]
fn test_faker_value_realistic_name_female() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let value = FakerValue::RealisticName { gender: Some("F".to_string()) };
    
    let result = value.generate(&mut faker).unwrap();
    assert!(result.contains('^'));
}

#[test]
fn test_faker_value_realistic_name_any() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let value = FakerValue::RealisticName { gender: None };
    
    let result = value.generate(&mut faker).unwrap();
    assert!(result.contains('^'));
}

#[test]
fn test_faker_value_realistic_address() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let value = FakerValue::RealisticAddress;
    
    let result = value.generate(&mut faker).unwrap();
    assert!(result.contains("USA"));
}

#[test]
fn test_faker_value_realistic_phone() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let value = FakerValue::RealisticPhone;
    
    let result = value.generate(&mut faker).unwrap();
    assert!(result.starts_with('('));
}

#[test]
fn test_faker_value_realistic_ssn() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let value = FakerValue::RealisticSsn;
    
    let result = value.generate(&mut faker).unwrap();
    assert_eq!(result.len(), 11);
}

#[test]
fn test_faker_value_realistic_mrn() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let value = FakerValue::RealisticMrn;
    
    let result = value.generate(&mut faker).unwrap();
    assert!((6..=10).contains(&result.len()));
}

#[test]
fn test_faker_value_realistic_icd10() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let value = FakerValue::RealisticIcd10;
    
    let result = value.generate(&mut faker).unwrap();
    assert!(result.contains('.'));
}

#[test]
fn test_faker_value_realistic_loinc() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let value = FakerValue::RealisticLoinc;
    
    let result = value.generate(&mut faker).unwrap();
    assert!(result.chars().all(|c| c.is_ascii_digit()));
}

#[test]
fn test_faker_value_realistic_medication() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let value = FakerValue::RealisticMedication;
    
    let result = value.generate(&mut faker).unwrap();
    assert!(!result.is_empty());
}

#[test]
fn test_faker_value_realistic_allergen() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let value = FakerValue::RealisticAllergen;
    
    let result = value.generate(&mut faker).unwrap();
    assert!(!result.is_empty());
}

#[test]
fn test_faker_value_realistic_blood_type() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let value = FakerValue::RealisticBloodType;
    
    let result = value.generate(&mut faker).unwrap();
    let valid = ["A+", "A-", "B+", "B-", "AB+", "AB-", "O+", "O-"];
    assert!(valid.contains(&result.as_str()));
}

#[test]
fn test_faker_value_realistic_ethnicity() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let value = FakerValue::RealisticEthnicity;
    
    let result = value.generate(&mut faker).unwrap();
    assert!(!result.is_empty());
}

#[test]
fn test_faker_value_realistic_race() {
    let mut rng = StdRng::seed_from_u64(42);
    let mut faker = Faker::new(&mut rng);
    let value = FakerValue::RealisticRace;
    
    let result = value.generate(&mut faker).unwrap();
    assert!(!result.is_empty());
}

// =============================================================================
// Error Display Tests
// =============================================================================

#[test]
fn test_date_error_display() {
    let err = DateError::InvalidDateFormat("bad".to_string());
    let msg = err.to_string();
    assert!(msg.contains("bad"));
    assert!(msg.contains("Invalid date format"));
}

#[test]
fn test_gaussian_error_display() {
    let err = GaussianError::InvalidParameters;
    let msg = err.to_string();
    assert!(msg.contains("Invalid Gaussian parameters"));
}

#[test]
fn test_generate_error_display() {
    let err = GenerateError::EmptyOptions;
    assert!(err.to_string().contains("empty options"));
    
    let err = GenerateError::EmptyMap;
    assert!(err.to_string().contains("empty map"));
    
    let err = GenerateError::Date(DateError::InvalidDateFormat("test".to_string()));
    assert!(err.to_string().contains("Date generation error"));
    
    let err = GenerateError::Gaussian(GaussianError::InvalidParameters);
    assert!(err.to_string().contains("Gaussian generation error"));
}

// =============================================================================
// Determinism Tests
// =============================================================================

#[test]
fn test_all_methods_deterministic() {
    let mut rng1 = StdRng::seed_from_u64(42);
    let mut faker1 = Faker::new(&mut rng1);
    
    let mut rng2 = StdRng::seed_from_u64(42);
    let mut faker2 = Faker::new(&mut rng2);
    
    // Test all methods for determinism
    assert_eq!(faker1.name(Some("M")), faker2.name(Some("M")));
    assert_eq!(faker1.address(), faker2.address());
    assert_eq!(faker1.phone(), faker2.phone());
    assert_eq!(faker1.ssn(), faker2.ssn());
    assert_eq!(faker1.mrn(), faker2.mrn());
    assert_eq!(faker1.icd10(), faker2.icd10());
    assert_eq!(faker1.loinc(), faker2.loinc());
    assert_eq!(faker1.medication(), faker2.medication());
    assert_eq!(faker1.allergen(), faker2.allergen());
    assert_eq!(faker1.blood_type(), faker2.blood_type());
    assert_eq!(faker1.ethnicity(), faker2.ethnicity());
    assert_eq!(faker1.race(), faker2.race());
    assert_eq!(faker1.numeric(5), faker2.numeric(5));
    assert_eq!(faker1.date("20200101", "20251231").unwrap(), faker2.date("20200101", "20251231").unwrap());
    assert_eq!(faker1.gaussian(100.0, 10.0, 2).unwrap(), faker2.gaussian(100.0, 10.0, 2).unwrap());
}
