//! HL7 v2 template value generation primitives.
//!
//! This crate owns the `ValueSource` domain model and concrete value generation used
//! by the template crate.

use crate::model::Error;
use crate::synthetic::faker::{Faker, FakerValue};
use rand::Rng;
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
        self.try_to_faker_value()
            .unwrap_or_else(|| FakerValue::Fixed(String::new()))
    }

    fn try_to_faker_value(&self) -> Option<FakerValue> {
        match self {
            Self::Fixed(value) => Some(FakerValue::Fixed(value.clone())),
            Self::From(options) => Some(FakerValue::From(options.clone())),
            Self::Numeric { digits } => Some(FakerValue::Numeric { digits: *digits }),
            Self::Date { start, end } => Some(FakerValue::Date {
                start: start.clone(),
                end: end.clone(),
            }),
            Self::Gaussian {
                mean,
                sd,
                precision,
            } => Some(FakerValue::Gaussian {
                mean: *mean,
                sd: *sd,
                precision: *precision,
            }),
            Self::Map(mapping) => Some(FakerValue::Map(mapping.clone())),
            Self::UuidV4 => Some(FakerValue::UuidV4),
            Self::DtmNowUtc => Some(FakerValue::DtmNowUtc),
            Self::RealisticName { gender } => Some(FakerValue::RealisticName {
                gender: gender.clone(),
            }),
            Self::RealisticAddress => Some(FakerValue::RealisticAddress),
            Self::RealisticPhone => Some(FakerValue::RealisticPhone),
            Self::RealisticSsn => Some(FakerValue::RealisticSsn),
            Self::RealisticMrn => Some(FakerValue::RealisticMrn),
            Self::RealisticIcd10 => Some(FakerValue::RealisticIcd10),
            Self::RealisticLoinc => Some(FakerValue::RealisticLoinc),
            Self::RealisticMedication => Some(FakerValue::RealisticMedication),
            Self::RealisticAllergen => Some(FakerValue::RealisticAllergen),
            Self::RealisticBloodType => Some(FakerValue::RealisticBloodType),
            Self::RealisticEthnicity => Some(FakerValue::RealisticEthnicity),
            Self::RealisticRace => Some(FakerValue::RealisticRace),
            Self::InvalidSegmentId
            | Self::InvalidFieldFormat
            | Self::InvalidRepFormat
            | Self::InvalidCompFormat
            | Self::InvalidSubcompFormat
            | Self::DuplicateDelims
            | Self::BadDelimLength => None,
        }
    }

    fn injected_error(&self) -> Option<Error> {
        match self {
            Self::InvalidSegmentId => Some(Error::InvalidSegmentId),
            Self::InvalidFieldFormat => Some(Error::InvalidFieldFormat {
                details: "Injected invalid field format".to_string(),
            }),
            Self::InvalidRepFormat => Some(Error::InvalidRepFormat {
                details: "Injected invalid repetition format".to_string(),
            }),
            Self::InvalidCompFormat => Some(Error::InvalidCompFormat {
                details: "Injected invalid component format".to_string(),
            }),
            Self::InvalidSubcompFormat => Some(Error::InvalidSubcompFormat {
                details: "Injected invalid subcomponent format".to_string(),
            }),
            Self::DuplicateDelims => Some(Error::DuplicateDelims),
            Self::BadDelimLength => Some(Error::BadDelimLength),
            _ => None,
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
    if let Some(error) = value_source.injected_error() {
        return Err(error);
    }

    if matches!(
        value_source,
        ValueSource::From(options) if options.is_empty()
    ) || matches!(
        value_source,
        ValueSource::Map(mapping) if mapping.is_empty()
    ) {
        return Ok(String::new());
    }

    generate_value_from_faker(value_source.to_faker_value(), rng)
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
    #![expect(
        clippy::panic,
        reason = "Value source unit tests fail explicitly on test setup errors."
    )]

    use super::{Error, ValueSource, generate_value};
    use crate::synthetic::faker::FakerValue;
    use rand::SeedableRng;
    use rand::rngs::StdRng;
    use std::collections::HashMap;

    fn seeded_rng() -> StdRng {
        StdRng::seed_from_u64(7)
    }

    #[test]
    fn fixed_returns_constant() {
        let mut rng = seeded_rng();
        let result = generate_value(&ValueSource::Fixed("hello".to_string()), &mut rng);
        assert_eq!(result.ok(), Some("hello".to_string()));
    }

    #[test]
    fn from_picks_one_of_options() {
        let mut rng = seeded_rng();
        let options = vec!["a".to_string(), "b".to_string(), "c".to_string()];
        let Ok(value) = generate_value(&ValueSource::From(options.clone()), &mut rng) else {
            panic!("from generation should succeed");
        };
        assert!(options.contains(&value), "{value} not in options");
    }

    #[test]
    fn from_empty_returns_empty_string() {
        let mut rng = seeded_rng();
        let result = generate_value(&ValueSource::From(Vec::new()), &mut rng);
        assert_eq!(result.ok(), Some(String::new()));
    }

    #[test]
    fn numeric_returns_exact_digit_count() {
        let mut rng = seeded_rng();
        let Ok(value) = generate_value(&ValueSource::Numeric { digits: 5 }, &mut rng) else {
            panic!("numeric should succeed");
        };
        assert_eq!(value.len(), 5);
        assert!(value.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn date_yields_yyyymmdd() {
        let mut rng = seeded_rng();
        let Ok(value) = generate_value(
            &ValueSource::Date {
                start: "20200101".to_string(),
                end: "20201231".to_string(),
            },
            &mut rng,
        ) else {
            panic!("date should succeed");
        };
        assert_eq!(value.len(), 8);
    }

    #[test]
    fn date_with_invalid_range_returns_escape_token_error() {
        let mut rng = seeded_rng();
        let result = generate_value(
            &ValueSource::Date {
                start: "20201231".to_string(),
                end: "20200101".to_string(),
            },
            &mut rng,
        );
        assert_eq!(result, Err(Error::InvalidEscapeToken));
    }

    #[test]
    fn gaussian_returns_formatted_decimal() {
        let mut rng = seeded_rng();
        let Ok(value) = generate_value(
            &ValueSource::Gaussian {
                mean: 50.0,
                sd: 5.0,
                precision: 3,
            },
            &mut rng,
        ) else {
            panic!("gaussian should succeed");
        };
        let parts: Vec<&str> = value.split('.').collect();
        assert_eq!(parts.len(), 2, "expected one decimal point in {value}");
        assert_eq!(parts.get(1).map(|s| s.len()), Some(3));
    }

    #[test]
    fn gaussian_invalid_returns_escape_token_error() {
        let mut rng = seeded_rng();
        // Non-finite standard deviation triggers Normal construction failure,
        // which maps through to Error::InvalidEscapeToken.
        let result = generate_value(
            &ValueSource::Gaussian {
                mean: 0.0,
                sd: f64::NAN,
                precision: 2,
            },
            &mut rng,
        );
        assert_eq!(result, Err(Error::InvalidEscapeToken));
    }

    #[test]
    fn map_returns_one_of_the_values() {
        let mut rng = seeded_rng();
        let mut map: HashMap<String, String> = HashMap::new();
        map.insert("k1".to_string(), "v1".to_string());
        map.insert("k2".to_string(), "v2".to_string());
        let Ok(value) = generate_value(&ValueSource::Map(map.clone()), &mut rng) else {
            panic!("map should succeed");
        };
        assert!(map.values().any(|v| v == &value));
    }

    #[test]
    fn map_empty_returns_empty_string() {
        let mut rng = seeded_rng();
        let result = generate_value(&ValueSource::Map(HashMap::new()), &mut rng);
        assert_eq!(result.ok(), Some(String::new()));
    }

    #[test]
    fn uuid_v4_has_36_chars() {
        let mut rng = seeded_rng();
        let Ok(value) = generate_value(&ValueSource::UuidV4, &mut rng) else {
            panic!("uuid generation should succeed");
        };
        assert_eq!(value.len(), 36);
    }

    #[test]
    fn dtm_now_utc_returns_14_digits() {
        let mut rng = seeded_rng();
        let Ok(value) = generate_value(&ValueSource::DtmNowUtc, &mut rng) else {
            panic!("dtm should succeed");
        };
        assert_eq!(value.len(), 14);
        assert!(value.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn realistic_name_male_is_caret_delimited() {
        let mut rng = seeded_rng();
        let Ok(value) = generate_value(
            &ValueSource::RealisticName {
                gender: Some("M".to_string()),
            },
            &mut rng,
        ) else {
            panic!("realistic name should succeed");
        };
        assert!(value.contains('^'));
    }

    #[test]
    fn realistic_name_none_gender_is_caret_delimited() {
        let mut rng = seeded_rng();
        let Ok(value) = generate_value(&ValueSource::RealisticName { gender: None }, &mut rng)
        else {
            panic!("realistic name should succeed");
        };
        assert!(value.contains('^'));
    }

    #[test]
    fn realistic_aliases_produce_non_empty_output() {
        let mut rng = seeded_rng();
        let sources = [
            ValueSource::RealisticAddress,
            ValueSource::RealisticPhone,
            ValueSource::RealisticSsn,
            ValueSource::RealisticMrn,
            ValueSource::RealisticIcd10,
            ValueSource::RealisticLoinc,
            ValueSource::RealisticMedication,
            ValueSource::RealisticAllergen,
            ValueSource::RealisticBloodType,
            ValueSource::RealisticEthnicity,
            ValueSource::RealisticRace,
        ];
        for source in &sources {
            let Ok(value) = generate_value(source, &mut rng) else {
                panic!("realistic source {source:?} should succeed");
            };
            assert!(
                !value.is_empty(),
                "{source:?} should produce non-empty output"
            );
        }
    }

    #[test]
    fn invalid_segment_id_returns_matching_error() {
        let mut rng = seeded_rng();
        let result = generate_value(&ValueSource::InvalidSegmentId, &mut rng);
        assert_eq!(result, Err(Error::InvalidSegmentId));
    }

    #[test]
    fn invalid_field_format_returns_matching_error() {
        let mut rng = seeded_rng();
        let result = generate_value(&ValueSource::InvalidFieldFormat, &mut rng);
        assert!(matches!(result, Err(Error::InvalidFieldFormat { .. })));
    }

    #[test]
    fn invalid_rep_format_returns_matching_error() {
        let mut rng = seeded_rng();
        let result = generate_value(&ValueSource::InvalidRepFormat, &mut rng);
        assert!(matches!(result, Err(Error::InvalidRepFormat { .. })));
    }

    #[test]
    fn invalid_comp_format_returns_matching_error() {
        let mut rng = seeded_rng();
        let result = generate_value(&ValueSource::InvalidCompFormat, &mut rng);
        assert!(matches!(result, Err(Error::InvalidCompFormat { .. })));
    }

    #[test]
    fn invalid_subcomp_format_returns_matching_error() {
        let mut rng = seeded_rng();
        let result = generate_value(&ValueSource::InvalidSubcompFormat, &mut rng);
        assert!(matches!(result, Err(Error::InvalidSubcompFormat { .. })));
    }

    #[test]
    fn duplicate_delims_returns_matching_error() {
        let mut rng = seeded_rng();
        let result = generate_value(&ValueSource::DuplicateDelims, &mut rng);
        assert_eq!(result, Err(Error::DuplicateDelims));
    }

    #[test]
    fn bad_delim_length_returns_matching_error() {
        let mut rng = seeded_rng();
        let result = generate_value(&ValueSource::BadDelimLength, &mut rng);
        assert_eq!(result, Err(Error::BadDelimLength));
    }

    #[test]
    fn to_faker_value_round_trips_known_variants() {
        let fixed = ValueSource::Fixed("x".to_string()).to_faker_value();
        assert_eq!(fixed, FakerValue::Fixed("x".to_string()));

        let from = ValueSource::From(vec!["a".to_string()]).to_faker_value();
        assert_eq!(from, FakerValue::From(vec!["a".to_string()]));

        let numeric = ValueSource::Numeric { digits: 4 }.to_faker_value();
        assert_eq!(numeric, FakerValue::Numeric { digits: 4 });

        let date = ValueSource::Date {
            start: "20200101".to_string(),
            end: "20201231".to_string(),
        }
        .to_faker_value();
        assert_eq!(
            date,
            FakerValue::Date {
                start: "20200101".to_string(),
                end: "20201231".to_string(),
            }
        );

        let gaussian = ValueSource::Gaussian {
            mean: 1.0,
            sd: 2.0,
            precision: 1,
        }
        .to_faker_value();
        assert_eq!(
            gaussian,
            FakerValue::Gaussian {
                mean: 1.0,
                sd: 2.0,
                precision: 1,
            }
        );

        let mut mapping = HashMap::new();
        mapping.insert("in".to_string(), "out".to_string());
        let map = ValueSource::Map(mapping.clone()).to_faker_value();
        assert_eq!(map, FakerValue::Map(mapping));

        let uuid = ValueSource::UuidV4.to_faker_value();
        assert_eq!(uuid, FakerValue::UuidV4);

        let dtm = ValueSource::DtmNowUtc.to_faker_value();
        assert_eq!(dtm, FakerValue::DtmNowUtc);

        let name = ValueSource::RealisticName {
            gender: Some("M".to_string()),
        }
        .to_faker_value();
        assert_eq!(
            name,
            FakerValue::RealisticName {
                gender: Some("M".to_string()),
            }
        );

        assert_eq!(
            ValueSource::RealisticAddress.to_faker_value(),
            FakerValue::RealisticAddress
        );
        assert_eq!(
            ValueSource::RealisticPhone.to_faker_value(),
            FakerValue::RealisticPhone
        );
        assert_eq!(
            ValueSource::RealisticSsn.to_faker_value(),
            FakerValue::RealisticSsn
        );
        assert_eq!(
            ValueSource::RealisticMrn.to_faker_value(),
            FakerValue::RealisticMrn
        );
        assert_eq!(
            ValueSource::RealisticIcd10.to_faker_value(),
            FakerValue::RealisticIcd10
        );
        assert_eq!(
            ValueSource::RealisticLoinc.to_faker_value(),
            FakerValue::RealisticLoinc
        );
        assert_eq!(
            ValueSource::RealisticMedication.to_faker_value(),
            FakerValue::RealisticMedication
        );
        assert_eq!(
            ValueSource::RealisticAllergen.to_faker_value(),
            FakerValue::RealisticAllergen
        );
        assert_eq!(
            ValueSource::RealisticBloodType.to_faker_value(),
            FakerValue::RealisticBloodType
        );
        assert_eq!(
            ValueSource::RealisticEthnicity.to_faker_value(),
            FakerValue::RealisticEthnicity
        );
        assert_eq!(
            ValueSource::RealisticRace.to_faker_value(),
            FakerValue::RealisticRace
        );

        // Error-injection variants map to an empty Fixed.
        let bad = ValueSource::BadDelimLength.to_faker_value();
        assert_eq!(bad, FakerValue::Fixed(String::new()));
    }
}
