//! Property-based tests for hl7v2-faker using proptest

use hl7v2_faker::{Faker, FakerValue, DateError, GaussianError, GenerateError};
use proptest::prelude::*;
use rand::SeedableRng;
use rand::rngs::StdRng;

// =============================================================================
// Name Property Tests
// =============================================================================

proptest! {
    #[test]
    fn test_name_format(seed: u64, gender in "M|F|") {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut faker = Faker::new(&mut rng);
        
        let gender_opt = if gender.is_empty() { None } else { Some(gender.as_str()) };
        let name = faker.name(gender_opt);
        
        // Should contain separator
        prop_assert!(name.contains('^'));
        
        // Should have exactly two parts
        let parts: Vec<&str> = name.split('^').collect();
        prop_assert_eq!(parts.len(), 2);
        prop_assert!(!parts[0].is_empty());
        prop_assert!(!parts[1].is_empty());
    }
    
    #[test]
    fn test_name_deterministic(seed: u64, gender in "M|F|") {
        let gender_opt = if gender.is_empty() { None } else { Some(gender.as_str()) };
        
        let mut rng1 = StdRng::seed_from_u64(seed);
        let mut faker1 = Faker::new(&mut rng1);
        let name1 = faker1.name(gender_opt);
        
        let mut rng2 = StdRng::seed_from_u64(seed);
        let mut faker2 = Faker::new(&mut rng2);
        let name2 = faker2.name(gender_opt);
        
        prop_assert_eq!(name1, name2);
    }
}

// =============================================================================
// Address Property Tests
// =============================================================================

proptest! {
    #[test]
    fn test_address_format(seed: u64) {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut faker = Faker::new(&mut rng);
        
        let address = faker.address();
        
        // Should contain separators
        prop_assert!(address.contains('^'));
        // Should contain country
        prop_assert!(address.contains("USA"));
    }
    
    #[test]
    fn test_address_deterministic(seed: u64) {
        let mut rng1 = StdRng::seed_from_u64(seed);
        let mut faker1 = Faker::new(&mut rng1);
        let addr1 = faker1.address();
        
        let mut rng2 = StdRng::seed_from_u64(seed);
        let mut faker2 = Faker::new(&mut rng2);
        let addr2 = faker2.address();
        
        prop_assert_eq!(addr1, addr2);
    }
}

// =============================================================================
// Phone Property Tests
// =============================================================================

proptest! {
    #[test]
    fn test_phone_format(seed: u64) {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut faker = Faker::new(&mut rng);
        
        let phone = faker.phone();
        
        // Should be 13 characters: (XXX)XXX-XXXX
        prop_assert_eq!(phone.len(), 13);
        
        // Should start with (
        prop_assert!(phone.starts_with('('));
        
        // Should contain )
        prop_assert!(phone.contains(')'));
        
        // Should contain -
        prop_assert!(phone.contains('-'));
    }
    
    #[test]
    fn test_phone_area_code_range(seed: u64) {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut faker = Faker::new(&mut rng);
        
        let phone = faker.phone();
        
        // Extract area code
        let area_code: String = phone.chars().skip(1).take(3).collect();
        let area: u32 = area_code.parse().unwrap();
        
        prop_assert!((200..999).contains(&area));
    }
}

// =============================================================================
// SSN Property Tests
// =============================================================================

proptest! {
    #[test]
    fn test_ssn_format(seed: u64) {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut faker = Faker::new(&mut rng);
        
        let ssn = faker.ssn();
        
        // Should be 11 characters: XXX-XX-XXXX
        prop_assert_eq!(ssn.len(), 11);
        
        // Should have dashes in correct positions
        prop_assert_eq!(&ssn[3..4], "-");
        prop_assert_eq!(&ssn[6..7], "-");
        
        // All non-dash characters should be digits
        for c in ssn.chars() {
            prop_assert!(c.is_ascii_digit() || c == '-');
        }
    }
}

// =============================================================================
// MRN Property Tests
// =============================================================================

proptest! {
    #[test]
    fn test_mrn_format(seed: u64) {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut faker = Faker::new(&mut rng);
        
        let mrn = faker.mrn();
        
        // Should be 6-10 characters
        prop_assert!((6..=10).contains(&mrn.len()));
        
        // All characters should be digits
        prop_assert!(mrn.chars().all(|c| c.is_ascii_digit()));
    }
}

// =============================================================================
// ICD-10 Property Tests
// =============================================================================

proptest! {
    #[test]
    fn test_icd10_format(seed: u64) {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut faker = Faker::new(&mut rng);
        
        let code = faker.icd10();
        
        // Should contain a dot
        prop_assert!(code.contains('.'));
        
        // Should start with a letter
        prop_assert!(code.chars().next().unwrap().is_ascii_uppercase());
        
        // Should be in format XXX.X
        let parts: Vec<&str> = code.split('.').collect();
        prop_assert_eq!(parts.len(), 2);
        prop_assert_eq!(parts[0].len(), 3);
        prop_assert_eq!(parts[1].len(), 1);
    }
}

// =============================================================================
// LOINC Property Tests
// =============================================================================

proptest! {
    #[test]
    fn test_loinc_format(seed: u64) {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut faker = Faker::new(&mut rng);
        
        let code = faker.loinc();
        
        // Should be all digits
        prop_assert!(code.chars().all(|c| c.is_ascii_digit()));
        
        // Should be 5-7 digits
        prop_assert!((5..=7).contains(&code.len()));
    }
}

// =============================================================================
// Blood Type Property Tests
// =============================================================================

proptest! {
    #[test]
    fn test_blood_type_valid(seed: u64) {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut faker = Faker::new(&mut rng);
        
        let blood_type = faker.blood_type();
        
        let valid_types = ["A+", "A-", "B+", "B-", "AB+", "AB-", "O+", "O-"];
        prop_assert!(valid_types.contains(&blood_type.as_str()));
    }
}

// =============================================================================
// Numeric Property Tests
// =============================================================================

proptest! {
    #[test]
    fn test_numeric_length(seed: u64, digits in 0usize..100) {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut faker = Faker::new(&mut rng);
        
        let num = faker.numeric(digits);
        
        prop_assert_eq!(num.len(), digits);
        prop_assert!(num.chars().all(|c| c.is_ascii_digit()));
    }
    
    #[test]
    fn test_numeric_deterministic(seed: u64, digits in 1usize..20) {
        let mut rng1 = StdRng::seed_from_u64(seed);
        let mut faker1 = Faker::new(&mut rng1);
        let num1 = faker1.numeric(digits);
        
        let mut rng2 = StdRng::seed_from_u64(seed);
        let mut faker2 = Faker::new(&mut rng2);
        let num2 = faker2.numeric(digits);
        
        prop_assert_eq!(num1, num2);
    }
}

// =============================================================================
// Date Property Tests
// =============================================================================

proptest! {
    #[test]
    fn test_date_in_range(seed: u64, start_year in 2000u32..2020, end_year in 2020u32..2030) {
        let start = format!("{}0101", start_year);
        let end = format!("{}1231", end_year);
        
        let mut rng = StdRng::seed_from_u64(seed);
        let mut faker = Faker::new(&mut rng);
        
        let date = faker.date(&start, &end).unwrap();
        
        // Should be 8 characters
        prop_assert_eq!(date.len(), 8);
        
        // Should be all digits
        prop_assert!(date.chars().all(|c| c.is_ascii_digit()));
        
        // Should be in range
        prop_assert!(date.as_str() >= start.as_str());
        prop_assert!(date.as_str() <= end.as_str());
    }
    
    #[test]
    fn test_date_deterministic(seed: u64) {
        let mut rng1 = StdRng::seed_from_u64(seed);
        let mut faker1 = Faker::new(&mut rng1);
        let date1 = faker1.date("20200101", "20251231").unwrap();
        
        let mut rng2 = StdRng::seed_from_u64(seed);
        let mut faker2 = Faker::new(&mut rng2);
        let date2 = faker2.date("20200101", "20251231").unwrap();
        
        prop_assert_eq!(date1, date2);
    }
}

// =============================================================================
// Gaussian Property Tests
// =============================================================================

proptest! {
    #[test]
    fn test_gaussian_reasonable_range(seed: u64, mean in -1000.0f64..1000.0, sd in 0.1f64..100.0, precision in 0usize..6) {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut faker = Faker::new(&mut rng);
        
        let value = faker.gaussian(mean, sd, precision).unwrap();
        
        // Should be parseable as f64
        let parsed: f64 = value.parse().unwrap();
        
        // Should be within 10 standard deviations (extremely likely)
        let lower = mean - 10.0 * sd;
        let upper = mean + 10.0 * sd;
        prop_assert!(parsed >= lower && parsed <= upper);
    }
    
    #[test]
    fn test_gaussian_deterministic(seed: u64, mean in 0.0f64..100.0, sd in 1.0f64..10.0, precision in 1usize..4) {
        let mut rng1 = StdRng::seed_from_u64(seed);
        let mut faker1 = Faker::new(&mut rng1);
        let value1 = faker1.gaussian(mean, sd, precision).unwrap();
        
        let mut rng2 = StdRng::seed_from_u64(seed);
        let mut faker2 = Faker::new(&mut rng2);
        let value2 = faker2.gaussian(mean, sd, precision).unwrap();
        
        prop_assert_eq!(value1, value2);
    }
}

// =============================================================================
// UUID Property Tests
// =============================================================================

proptest! {
    #[test]
    fn test_uuid_format(seed: u64) {
        let mut rng = StdRng::seed_from_u64(seed);
        let faker = Faker::new(&mut rng);
        
        let uuid = faker.uuid_v4();
        
        // Should be 36 characters
        prop_assert_eq!(uuid.len(), 36);
        
        // Should have 4 dashes
        prop_assert_eq!(uuid.matches('-').count(), 4);
        
        // Dashes should be at correct positions
        prop_assert_eq!(&uuid[8..9], "-");
        prop_assert_eq!(&uuid[13..14], "-");
        prop_assert_eq!(&uuid[18..19], "-");
        prop_assert_eq!(&uuid[23..24], "-");
    }
    
    #[test]
    fn test_uuid_uniqueness(_seed: u64) {
        let mut rng = StdRng::seed_from_u64(42);
        let faker = Faker::new(&mut rng);
        
        // Generate 100 UUIDs
        let uuids: Vec<String> = (0..100).map(|_| faker.uuid_v4()).collect();
        
        // All should be unique
        let unique: std::collections::HashSet<_> = uuids.iter().collect();
        prop_assert_eq!(unique.len(), 100);
    }
}

// =============================================================================
// FakerValue Property Tests
// =============================================================================

proptest! {
    #[test]
    fn test_faker_value_fixed(seed: u64, value in ".*") {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut faker = Faker::new(&mut rng);
        
        let fv = FakerValue::Fixed(value.clone());
        let result = fv.generate(&mut faker).unwrap();
        
        prop_assert_eq!(result, value);
    }
    
    #[test]
    fn test_faker_value_numeric(seed: u64, digits in 0usize..20) {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut faker = Faker::new(&mut rng);
        
        let fv = FakerValue::Numeric { digits };
        let result = fv.generate(&mut faker).unwrap();
        
        prop_assert_eq!(result.len(), digits);
        prop_assert!(result.chars().all(|c| c.is_ascii_digit()));
    }
    
    #[test]
    fn test_faker_value_from(seed: u64, options in prop::collection::vec(".*", 1..10)) {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut faker = Faker::new(&mut rng);
        
        let fv = FakerValue::From(options.clone());
        let result = fv.generate(&mut faker).unwrap();
        
        prop_assert!(options.contains(&result));
    }
}

// =============================================================================
// Select From Property Tests
// =============================================================================

proptest! {
    #[test]
    fn test_select_from_always_in_options(seed: u64, options in prop::collection::vec(".*", 1..20)) {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut faker = Faker::new(&mut rng);
        
        let result = faker.select_from(&options).unwrap();
        
        prop_assert!(options.contains(&result));
    }
    
    #[test]
    fn test_select_from_empty_returns_none(seed: u64) {
        let mut rng = StdRng::seed_from_u64(seed);
        let mut faker = Faker::new(&mut rng);
        
        let options: Vec<String> = vec![];
        let result = faker.select_from(&options);
        
        prop_assert!(result.is_none());
    }
}

// =============================================================================
// Determinism Property Tests
// =============================================================================

proptest! {
    #[test]
    fn test_all_methods_deterministic(seed: u64) {
        let mut rng1 = StdRng::seed_from_u64(seed);
        let mut faker1 = Faker::new(&mut rng1);
        
        let mut rng2 = StdRng::seed_from_u64(seed);
        let mut faker2 = Faker::new(&mut rng2);
        
        // Test all methods
        prop_assert_eq!(faker1.name(None), faker2.name(None));
        prop_assert_eq!(faker1.address(), faker2.address());
        prop_assert_eq!(faker1.phone(), faker2.phone());
        prop_assert_eq!(faker1.ssn(), faker2.ssn());
        prop_assert_eq!(faker1.mrn(), faker2.mrn());
        prop_assert_eq!(faker1.icd10(), faker2.icd10());
        prop_assert_eq!(faker1.loinc(), faker2.loinc());
        prop_assert_eq!(faker1.medication(), faker2.medication());
        prop_assert_eq!(faker1.allergen(), faker2.allergen());
        prop_assert_eq!(faker1.blood_type(), faker2.blood_type());
        prop_assert_eq!(faker1.ethnicity(), faker2.ethnicity());
        prop_assert_eq!(faker1.race(), faker2.race());
        prop_assert_eq!(faker1.numeric(5), faker2.numeric(5));
        prop_assert_eq!(
            faker1.date("20200101", "20251231").unwrap(),
            faker2.date("20200101", "20251231").unwrap()
        );
        prop_assert_eq!(
            faker1.gaussian(100.0, 10.0, 2).unwrap(),
            faker2.gaussian(100.0, 10.0, 2).unwrap()
        );
    }
}
