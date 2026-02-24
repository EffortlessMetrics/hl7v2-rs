//! HL7 v2 template value generation primitives.
//!
//! This crate owns the `ValueSource` domain model and concrete value generation used
//! by the template crate.

use hl7v2_core::Error;
use hl7v2_faker::{Faker, FakerValue};
use rand::Rng;
use rand::RngExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Source for generating values in a field/rep/component template.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum ValueSource {
    /// A fixed constant value.
    Fixed(String),
    /// A random choice from a list of options.
    From(Vec<String>),
    /// A random numeric string with specified number of digits.
    Numeric {
        /// Number of digits in output.
        digits: usize,
    },
    /// A random date within a range (YYYYMMDD format).
    Date {
        /// Start date in YYYYMMDD format.
        start: String,
        /// End date in YYYYMMDD format.
        end: String,
    },
    /// A Gaussian-distributed numeric value.
    Gaussian {
        /// Distribution mean.
        mean: f64,
        /// Distribution standard deviation.
        sd: f64,
        /// Number of decimal places.
        precision: usize,
    },
    /// A value mapped from a key-value map.
    Map(HashMap<String, String>),
    /// Random UUID v4.
    UuidV4,
    /// Current UTC timestamp in YYYYMMDDHHMMSS format.
    DtmNowUtc,
    /// Realistic person name (optionally filtered by gender: "M", "F", or None).
    RealisticName {
        /// Optional gender filter.
        gender: Option<String>,
    },
    /// Realistic street address.
    RealisticAddress,
    /// Realistic phone number.
    RealisticPhone,
    /// Realistic Social Security Number.
    RealisticSsn,
    /// Realistic Medical Record Number.
    RealisticMrn,
    /// Realistic ICD-10 diagnosis code.
    RealisticIcd10,
    /// Realistic LOINC observation code.
    RealisticLoinc,
    /// Realistic medication name.
    RealisticMedication,
    /// Realistic allergen name.
    RealisticAllergen,
    /// Realistic blood type.
    RealisticBloodType,
    /// Realistic ethnicity code.
    RealisticEthnicity,
    /// Realistic race code.
    RealisticRace,
    /// Injects an invalid segment ID error.
    InvalidSegmentId,
    /// Injects an invalid field format error.
    InvalidFieldFormat,
    /// Injects an invalid repetition format error.
    InvalidRepFormat,
    /// Injects an invalid component format error.
    InvalidCompFormat,
    /// Injects an invalid subcomponent format error.
    InvalidSubcompFormat,
    /// Injects a duplicate delimiters error.
    DuplicateDelims,
    /// Injects a bad delimiter length error.
    BadDelimLength,
}

impl ValueSource {
    /// Convert to a `FakerValue` for callers that still operate on faker types.
    pub fn to_faker_value(&self) -> FakerValue {
        match self {
            Self::Fixed(value) => FakerValue::Fixed(value.clone()),
            Self::From(options) => FakerValue::From(options.clone()),
            Self::Numeric { digits } => FakerValue::Numeric {
                digits: *digits,
            },
            Self::Date { start, end } => FakerValue::Date {
                start: start.clone(),
                end: end.clone(),
            },
            Self::Gaussian { mean, sd, precision } => FakerValue::Gaussian {
                mean: *mean,
                sd: *sd,
                precision: *precision,
            },
            Self::Map(mapping) => FakerValue::Map(mapping.clone()),
            Self::UuidV4 => FakerValue::UuidV4,
            Self::DtmNowUtc => FakerValue::DtmNowUtc,
            Self::RealisticName { gender } => FakerValue::RealisticName {
                gender: gender.clone(),
            },
            Self::RealisticAddress => FakerValue::RealisticAddress,
            Self::RealisticPhone => FakerValue::RealisticPhone,
            Self::RealisticSsn => FakerValue::RealisticSsn,
            Self::RealisticMrn => FakerValue::RealisticMrn,
            Self::RealisticIcd10 => FakerValue::RealisticIcd10,
            Self::RealisticLoinc => FakerValue::RealisticLoinc,
            Self::RealisticMedication => FakerValue::RealisticMedication,
            Self::RealisticAllergen => FakerValue::RealisticAllergen,
            Self::RealisticBloodType => FakerValue::RealisticBloodType,
            Self::RealisticEthnicity => FakerValue::RealisticEthnicity,
            Self::RealisticRace => FakerValue::RealisticRace,
            _ => FakerValue::Fixed(String::new()),
        }
    }
}

/// Generate a concrete string value for a configured value source.
pub fn generate_value<R: Rng>(
    value_source: &ValueSource,
    rng: &mut R,
) -> Result<String, Error> {
    match value_source {
        ValueSource::Fixed(value) => Ok(value.clone()),
        ValueSource::From(options) => {
            if options.is_empty() {
                return Ok(String::new());
            }
            let index = rng.random_range(0..options.len());
            Ok(options[index].clone())
        }
        ValueSource::Numeric { digits } => {
            let mut faker = Faker::new(rng);
            Ok(faker.numeric(*digits))
        }
        ValueSource::Map(mapping) => {
            if mapping.is_empty() {
                return Ok(String::new());
            }
            let value_source = FakerValue::Map(mapping.clone());
            generate_value_from_faker(value_source, rng)
        }
        ValueSource::Date { start, end } => {
            let value_source = FakerValue::Date {
                start: start.clone(),
                end: end.clone(),
            };
            generate_value_from_faker(value_source, rng)
        }
        ValueSource::Gaussian {
            mean,
            sd,
            precision,
        } => {
            let value_source = FakerValue::Gaussian {
                mean: *mean,
                sd: *sd,
                precision: *precision,
            };
            generate_value_from_faker(value_source, rng)
        }
        ValueSource::UuidV4 => generate_value_from_faker(FakerValue::UuidV4, rng),
        ValueSource::DtmNowUtc => generate_value_from_faker(FakerValue::DtmNowUtc, rng),
        ValueSource::RealisticName { gender } => generate_value_from_faker(
            FakerValue::RealisticName {
                gender: gender.clone(),
            },
            rng,
        ),
        ValueSource::RealisticAddress => {
            generate_value_from_faker(FakerValue::RealisticAddress, rng)
        }
        ValueSource::RealisticPhone => {
            generate_value_from_faker(FakerValue::RealisticPhone, rng)
        }
        ValueSource::RealisticSsn => generate_value_from_faker(FakerValue::RealisticSsn, rng),
        ValueSource::RealisticMrn => generate_value_from_faker(FakerValue::RealisticMrn, rng),
        ValueSource::RealisticIcd10 => generate_value_from_faker(FakerValue::RealisticIcd10, rng),
        ValueSource::RealisticLoinc => generate_value_from_faker(FakerValue::RealisticLoinc, rng),
        ValueSource::RealisticMedication => {
            generate_value_from_faker(FakerValue::RealisticMedication, rng)
        }
        ValueSource::RealisticAllergen => {
            generate_value_from_faker(FakerValue::RealisticAllergen, rng)
        }
        ValueSource::RealisticBloodType => {
            generate_value_from_faker(FakerValue::RealisticBloodType, rng)
        }
        ValueSource::RealisticEthnicity => {
            generate_value_from_faker(FakerValue::RealisticEthnicity, rng)
        }
        ValueSource::RealisticRace => {
            generate_value_from_faker(FakerValue::RealisticRace, rng)
        }
        ValueSource::InvalidSegmentId => Err(Error::InvalidSegmentId),
        ValueSource::InvalidFieldFormat => Err(Error::InvalidFieldFormat {
            details: "Injected invalid field format".to_string(),
        }),
        ValueSource::InvalidRepFormat => Err(Error::InvalidRepFormat {
            details: "Injected invalid repetition format".to_string(),
        }),
        ValueSource::InvalidCompFormat => Err(Error::InvalidCompFormat {
            details: "Injected invalid component format".to_string(),
        }),
        ValueSource::InvalidSubcompFormat => Err(Error::InvalidSubcompFormat {
            details: "Injected invalid subcomponent format".to_string(),
        }),
        ValueSource::DuplicateDelims => Err(Error::DuplicateDelims),
        ValueSource::BadDelimLength => Err(Error::BadDelimLength),
    }
}

fn generate_value_from_faker<R: Rng>(
    value_source: FakerValue,
    rng: &mut R,
) -> Result<String, Error> {
    let mut faker = Faker::new(rng);
    value_source
        .generate(&mut faker)
        .map_err(|_| Error::InvalidEscapeToken)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use proptest::prelude::*;

    #[test]
    fn test_fixed_value() {
        let mut rng = StdRng::seed_from_u64(42);
        let source = ValueSource::Fixed("abc".to_string());
        assert_eq!(generate_value(&source, &mut rng).unwrap(), "abc");
    }

    #[test]
    fn test_numeric_value_length() {
        let mut rng = StdRng::seed_from_u64(42);
        let source = ValueSource::Numeric { digits: 5 };
        let value = generate_value(&source, &mut rng).unwrap();
        assert_eq!(value.len(), 5);
        assert!(value.bytes().all(|b| b.is_ascii_digit()));
    }

    #[test]
    fn test_date_range_generation() {
        let mut rng = StdRng::seed_from_u64(42);
        let source = ValueSource::Date {
            start: "20200101".to_string(),
            end: "20200131".to_string(),
        };
        let value = generate_value(&source, &mut rng).unwrap();
        assert!(chrono::NaiveDate::parse_from_str(&value, "%Y%m%d").is_ok());
    }

    #[test]
    fn test_to_faker_value_includes_injection_fallback() {
        let source = ValueSource::InvalidSegmentId;
        assert_eq!(source.to_faker_value(), FakerValue::Fixed(String::new()));
    }

    #[test]
    fn test_injection_error() {
        let mut rng = StdRng::seed_from_u64(42);
        let source = ValueSource::InvalidSegmentId;
        assert!(generate_value(&source, &mut rng).is_err());
    }

    proptest::proptest! {
        #[test]
        fn numeric_length_is_stable(digits in 0usize..8) {
            let mut rng = StdRng::seed_from_u64(2026);
            let source = ValueSource::Numeric { digits };
            let value = generate_value(&source, &mut rng).unwrap();
            prop_assert_eq!(value.len(), digits);
            for ch in value.bytes() {
                prop_assert!(ch.is_ascii_digit());
            }
        }

        #[test]
        fn from_value_chooses_known_option(
            value1 in "[a-z]{1,8}",
            value2 in "[a-z]{1,8}",
            seed in 0u64..1_000_000,
        ) {
            let mut rng = StdRng::seed_from_u64(seed);
            let source = ValueSource::From(vec![value1.clone(), value2.clone()]);
            let generated = generate_value(&source, &mut rng).unwrap();
            prop_assert!(generated == value1 || generated == value2);
        }
    }
}
