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
