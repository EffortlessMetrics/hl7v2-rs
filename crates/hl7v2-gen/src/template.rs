use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Message template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub name: String,
    pub delims: String,
    pub segments: Vec<String>,
    #[serde(default)]
    pub values: HashMap<String, Vec<ValueSource>>,
}

/// Source for generating values
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum ValueSource {
    Fixed(String),
    From(Vec<String>),
    Numeric { digits: usize },
    Date { start: String, end: String },
    Gaussian { mean: f64, sd: f64, precision: usize },
    Map(HashMap<String, String>),
    UuidV4,
    DtmNowUtc,
    // Realistic data generation variants
    RealisticName { gender: Option<String> }, // "M", "F", or None for any
    RealisticAddress,
    RealisticPhone,
    RealisticSsn,
    RealisticMrn, // Medical Record Number
    RealisticIcd10,
    RealisticLoinc,
    RealisticMedication,
    RealisticAllergen,
    RealisticBloodType,
    RealisticEthnicity,
    RealisticRace,
    // Error injection variants for negative testing
    InvalidSegmentId,
    InvalidFieldFormat,
    InvalidRepFormat,
    InvalidCompFormat,
    InvalidSubcompFormat,
    DuplicateDelims,
    BadDelimLength,
}
