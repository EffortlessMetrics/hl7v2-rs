//! HL7 v2 template value generation primitives.
//!
//! This crate owns the `ValueSource` domain model and concrete value generation used
//! by the template crate.

use crate::model::Error;
use crate::synthetic::faker::{Faker, FakerValue};
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
            Self::Numeric { digits } => FakerValue::Numeric { digits: *digits },
            Self::Date { start, end } => FakerValue::Date {
                start: start.clone(),
                end: end.clone(),
            },
            Self::Gaussian {
                mean,
                sd,
                precision,
            } => FakerValue::Gaussian {
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
///
/// # Errors
///
/// Returns parser/model errors when the selected source intentionally injects
/// invalid HL7 content or when faker-backed value generation fails.
pub fn generate_value<R: Rng>(value_source: &ValueSource, rng: &mut R) -> Result<String, Error> {
    match value_source {
        ValueSource::Fixed(value) => Ok(value.clone()),
        ValueSource::From(options) => {
            if options.is_empty() {
                return Ok(String::new());
            }
            let index = rng.random_range(0..options.len());
            Ok(options.get(index).cloned().unwrap_or_default())
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
        ValueSource::RealisticPhone => generate_value_from_faker(FakerValue::RealisticPhone, rng),
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
        ValueSource::RealisticRace => generate_value_from_faker(FakerValue::RealisticRace, rng),
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
        .map_err(|_err| Error::InvalidEscapeToken)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn ensure(condition: bool, message: &'static str) -> TestResult {
        if condition {
            Ok(())
        } else {
            Err(std::io::Error::other(message).into())
        }
    }

    fn seeded_rng() -> StdRng {
        StdRng::seed_from_u64(42)
    }

    #[test]
    fn generate_value_fixed_returns_literal() -> TestResult {
        let mut rng = seeded_rng();
        let result = generate_value(&ValueSource::Fixed("HELLO".to_string()), &mut rng)?;
        ensure(result == "HELLO", "fixed value did not round-trip")
    }

    #[test]
    fn generate_value_from_empty_returns_empty_string() -> TestResult {
        let mut rng = seeded_rng();
        let result = generate_value(&ValueSource::From(Vec::new()), &mut rng)?;
        ensure(result.is_empty(), "empty From should yield empty string")
    }

    #[test]
    fn generate_value_from_picks_one_of_options() -> TestResult {
        let mut rng = seeded_rng();
        let options = vec!["alpha".to_string(), "beta".to_string()];
        let result = generate_value(&ValueSource::From(options.clone()), &mut rng)?;
        ensure(options.contains(&result), "From did not pick an option")
    }

    #[test]
    fn generate_value_numeric_returns_string_of_digits_with_expected_length() -> TestResult {
        let mut rng = seeded_rng();
        let result = generate_value(&ValueSource::Numeric { digits: 5 }, &mut rng)?;
        ensure(result.len() == 5, "numeric length should match digits")?;
        ensure(
            result.chars().all(|c| c.is_ascii_digit()),
            "numeric should contain only digits",
        )
    }

    #[test]
    fn generate_value_map_empty_returns_empty_string() -> TestResult {
        let mut rng = seeded_rng();
        let result = generate_value(&ValueSource::Map(HashMap::new()), &mut rng)?;
        ensure(result.is_empty(), "empty Map should yield empty string")
    }

    #[test]
    fn generate_value_uuid_v4_returns_uuid_shaped_string() -> TestResult {
        let mut rng = seeded_rng();
        let result = generate_value(&ValueSource::UuidV4, &mut rng)?;
        ensure(result.len() == 36, "UUID should be 36 chars")?;
        let bytes = result.as_bytes();
        for index in [8usize, 13, 18, 23] {
            ensure(
                bytes.get(index).copied() == Some(b'-'),
                "UUID should have hyphen at expected position",
            )?;
        }
        Ok(())
    }

    #[test]
    fn generate_value_dtm_now_utc_returns_fourteen_digit_string() -> TestResult {
        let mut rng = seeded_rng();
        let result = generate_value(&ValueSource::DtmNowUtc, &mut rng)?;
        ensure(result.len() == 14, "DTM should be 14 chars")?;
        ensure(
            result.chars().all(|c| c.is_ascii_digit()),
            "DTM should contain only digits",
        )
    }

    #[test]
    fn generate_value_date_degenerate_range_returns_only_start() -> TestResult {
        let mut rng = seeded_rng();
        let result = generate_value(
            &ValueSource::Date {
                start: "20200101".to_string(),
                end: "20200101".to_string(),
            },
            &mut rng,
        )?;
        ensure(result == "20200101", "degenerate range should yield start")
    }

    #[test]
    fn generate_value_gaussian_zero_sd_returns_mean_at_precision() -> TestResult {
        let mut rng = seeded_rng();
        let result = generate_value(
            &ValueSource::Gaussian {
                mean: 100.0,
                sd: 0.0,
                precision: 2,
            },
            &mut rng,
        )?;
        ensure(
            result == "100.00",
            "Gaussian with zero SD should equal mean",
        )
    }

    #[test]
    fn generate_value_invalid_segment_id_returns_matching_error() -> TestResult {
        let mut rng = seeded_rng();
        let result = generate_value(&ValueSource::InvalidSegmentId, &mut rng);
        ensure(
            matches!(result, Err(Error::InvalidSegmentId)),
            "should return InvalidSegmentId",
        )
    }

    #[test]
    fn generate_value_invalid_field_format_returns_matching_error() -> TestResult {
        let mut rng = seeded_rng();
        let result = generate_value(&ValueSource::InvalidFieldFormat, &mut rng);
        ensure(
            matches!(result, Err(Error::InvalidFieldFormat { .. })),
            "should return InvalidFieldFormat",
        )
    }

    #[test]
    fn generate_value_invalid_rep_format_returns_matching_error() -> TestResult {
        let mut rng = seeded_rng();
        let result = generate_value(&ValueSource::InvalidRepFormat, &mut rng);
        ensure(
            matches!(result, Err(Error::InvalidRepFormat { .. })),
            "should return InvalidRepFormat",
        )
    }

    #[test]
    fn generate_value_invalid_comp_format_returns_matching_error() -> TestResult {
        let mut rng = seeded_rng();
        let result = generate_value(&ValueSource::InvalidCompFormat, &mut rng);
        ensure(
            matches!(result, Err(Error::InvalidCompFormat { .. })),
            "should return InvalidCompFormat",
        )
    }

    #[test]
    fn generate_value_invalid_subcomp_format_returns_matching_error() -> TestResult {
        let mut rng = seeded_rng();
        let result = generate_value(&ValueSource::InvalidSubcompFormat, &mut rng);
        ensure(
            matches!(result, Err(Error::InvalidSubcompFormat { .. })),
            "should return InvalidSubcompFormat",
        )
    }

    #[test]
    fn generate_value_duplicate_delims_returns_matching_error() -> TestResult {
        let mut rng = seeded_rng();
        let result = generate_value(&ValueSource::DuplicateDelims, &mut rng);
        ensure(
            matches!(result, Err(Error::DuplicateDelims)),
            "should return DuplicateDelims",
        )
    }

    #[test]
    fn generate_value_bad_delim_length_returns_matching_error() -> TestResult {
        let mut rng = seeded_rng();
        let result = generate_value(&ValueSource::BadDelimLength, &mut rng);
        ensure(
            matches!(result, Err(Error::BadDelimLength)),
            "should return BadDelimLength",
        )
    }

    #[test]
    fn to_faker_value_fixed_round_trips() -> TestResult {
        let source = ValueSource::Fixed("X".to_string());
        ensure(
            matches!(source.to_faker_value(), FakerValue::Fixed(ref value) if value == "X"),
            "Fixed did not round-trip to FakerValue::Fixed",
        )
    }

    #[test]
    fn to_faker_value_from_round_trips() -> TestResult {
        let source = ValueSource::From(vec!["a".to_string(), "b".to_string()]);
        ensure(
            matches!(source.to_faker_value(), FakerValue::From(ref options) if options.len() == 2),
            "From did not round-trip to FakerValue::From",
        )
    }

    #[test]
    fn to_faker_value_numeric_round_trips() -> TestResult {
        let source = ValueSource::Numeric { digits: 7 };
        ensure(
            matches!(source.to_faker_value(), FakerValue::Numeric { digits: 7 }),
            "Numeric did not round-trip to FakerValue::Numeric",
        )
    }

    #[test]
    fn to_faker_value_realistic_name_with_gender_round_trips() -> TestResult {
        let source = ValueSource::RealisticName {
            gender: Some("M".to_string()),
        };
        ensure(
            matches!(
                source.to_faker_value(),
                FakerValue::RealisticName { gender: Some(ref g) } if g == "M"
            ),
            "RealisticName did not round-trip to FakerValue::RealisticName",
        )
    }

    #[test]
    fn to_faker_value_uuid_v4_round_trips() -> TestResult {
        let source = ValueSource::UuidV4;
        ensure(
            matches!(source.to_faker_value(), FakerValue::UuidV4),
            "UuidV4 did not round-trip to FakerValue::UuidV4",
        )
    }

    #[test]
    fn to_faker_value_error_variant_falls_back_to_fixed_empty() -> TestResult {
        let source = ValueSource::InvalidSegmentId;
        ensure(
            matches!(source.to_faker_value(), FakerValue::Fixed(ref value) if value.is_empty()),
            "error variant should fall back to FakerValue::Fixed(empty)",
        )
    }

    #[test]
    fn value_source_fixed_serde_round_trip() -> TestResult {
        let source = ValueSource::Fixed("hello".to_string());
        let encoded = serde_json::to_string(&source)?;
        let decoded: ValueSource = serde_json::from_str(&encoded)?;
        ensure(
            matches!(decoded, ValueSource::Fixed(ref value) if value == "hello"),
            "serde round-trip should preserve Fixed value",
        )
    }
}
