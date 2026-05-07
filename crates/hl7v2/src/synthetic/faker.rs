//! Realistic HL7 v2 test data generation.
//!
//! This module provides faker-style data generation for creating realistic
//! HL7 v2 test data. It generates names, addresses, medical codes, and
//! other healthcare-related test data.
//!
//! # Features
//!
//! - **Name generation**: First/last names with gender-specific options
//! - **Address generation**: Streets, cities, states, zip codes
//! - **Medical codes**: ICD-10, LOINC, medications, allergens
//! - **Patient data**: MRN, SSN, blood type, race, ethnicity
//! - **Contact info**: Phone numbers
//! - **Date/time**: Date ranges, timestamps
//! - **Identifiers**: UUID v4
//!
//! # Example
//!
//! ```
//! use hl7v2::synthetic::faker::{Faker, FakerValue};
//! use rand::SeedableRng;
//! use rand::rngs::StdRng;
//!
//! // Create a seeded faker for deterministic output
//! let mut rng = StdRng::seed_from_u64(42);
//! let mut faker = Faker::new(&mut rng);
//!
//! // Generate realistic patient data
//! let name = faker.name(Some("M"));  // Male name
//! let address = faker.address();
//! let phone = faker.phone();
//! let mrn = faker.mrn();
//! ```

use rand::RngExt;
use rand_distr::Normal;

fn random_copy<T: Copy, R: Rng>(rng: &mut R, values: &[T]) -> Option<T> {
    if values.is_empty() {
        return None;
    }

    values.get(rng.random_range(0..values.len())).copied()
}

/// Main faker struct for generating realistic test data.
pub struct Faker<'a, R: Rng> {
    rng: &'a mut R,
}

impl<'a, R: Rng> Faker<'a, R> {
    /// Create a new faker instance with the given random number generator.
    pub fn new(rng: &'a mut R) -> Self {
        Self { rng }
    }

    /// Generate a realistic name in HL7 format (LAST^FIRST).
    ///
    /// # Arguments
    ///
    /// * `gender` - Optional gender ("M" for male, "F" for female, None for any)
    ///
    /// # Returns
    ///
    /// A name string in the format "LASTNAME^FIRSTNAME"
    pub fn name(&mut self, gender: Option<&str>) -> String {
        let first_names = match gender {
            Some("M") => &[
                "James", "John", "Robert", "Michael", "William", "David", "Richard", "Joseph",
                "Thomas", "Charles",
            ][..],
            Some("F") => &[
                "Mary",
                "Patricia",
                "Jennifer",
                "Linda",
                "Elizabeth",
                "Barbara",
                "Susan",
                "Jessica",
                "Sarah",
                "Karen",
            ][..],
            _ => &[
                "James",
                "Mary",
                "John",
                "Patricia",
                "Robert",
                "Jennifer",
                "Michael",
                "Linda",
                "William",
                "Elizabeth",
                "David",
                "Barbara",
                "Richard",
                "Susan",
                "Joseph",
                "Jessica",
            ][..],
        };

        let last_names = &[
            "Smith",
            "Johnson",
            "Williams",
            "Brown",
            "Jones",
            "Garcia",
            "Miller",
            "Davis",
            "Rodriguez",
            "Martinez",
            "Hernandez",
            "Lopez",
            "Gonzalez",
            "Wilson",
            "Anderson",
        ];

        let first_name = random_copy(self.rng, first_names).unwrap_or("");
        let last_name = random_copy(self.rng, last_names).unwrap_or("");

        format!("{last_name}^{first_name}")
    }

    /// Generate a realistic address in HL7 format.
    ///
    /// # Returns
    ///
    /// An address string in HL7 format: "STREET^CITY^STATE^ZIP^COUNTRY"
    pub fn address(&mut self) -> String {
        let streets = &[
            "Main St",
            "Oak Ave",
            "Pine Rd",
            "Elm St",
            "Maple Dr",
            "Cedar Ln",
            "Birch Way",
            "Washington St",
            "Lake St",
            "Hill St",
        ];

        let cities = &[
            "Anytown",
            "Springfield",
            "Riverside",
            "Fairview",
            "Centerville",
            "Georgetown",
            "Mount Pleasant",
            "Oakland",
            "Middletown",
            "Franklin",
        ];

        let states = &["AL", "AK", "AZ", "AR", "CA", "CO", "CT", "DE", "FL", "GA"];

        let street_number = self.rng.random_range(100..9999);
        let street = random_copy(self.rng, streets).unwrap_or("");
        let city = random_copy(self.rng, cities).unwrap_or("");
        let state = random_copy(self.rng, states).unwrap_or("");
        let zip = format!("{:05}", self.rng.random_range(10000..99999));

        format!(
            "{} {}^^{}^{}^{}^{}",
            street_number, street, city, state, zip, "USA"
        )
    }

    /// Generate a realistic phone number.
    ///
    /// # Returns
    ///
    /// A phone number in the format "(AREA)EXCHANGE-NUMBER"
    pub fn phone(&mut self) -> String {
        let area_code = self.rng.random_range(200..999);
        let exchange = self.rng.random_range(200..999);
        let number = self.rng.random_range(1000..9999);
        format!("({area_code}){exchange}-{number}")
    }

    /// Generate a realistic Social Security Number.
    ///
    /// # Returns
    ///
    /// An SSN in the format "XXX-XX-XXXX"
    pub fn ssn(&mut self) -> String {
        let part1 = self.rng.random_range(100..999);
        let part2 = self.rng.random_range(10..99);
        let part3 = self.rng.random_range(1000..9999);
        format!("{part1}-{part2}-{part3}")
    }

    /// Generate a realistic Medical Record Number.
    ///
    /// # Returns
    ///
    /// An MRN with 6-10 digits
    pub fn mrn(&mut self) -> String {
        let length = self.rng.random_range(6..=10);
        let mut mrn = String::new();
        for _ in 0..length {
            let digit = self.rng.random_range(0..10);
            mrn.push_str(&digit.to_string());
        }
        mrn
    }

    /// Generate a realistic ICD-10 diagnosis code.
    ///
    /// # Returns
    ///
    /// An ICD-10 code in the format "XXX.X"
    pub fn icd10(&mut self) -> String {
        let categories = &[
            "A00", "B01", "C02", "D03", "E04", "F05", "G06", "H07", "I08", "J09",
        ];
        let category = random_copy(self.rng, categories).unwrap_or("");
        let subcode = self.rng.random_range(0..10);
        format!("{category}.{subcode}")
    }

    /// Generate a realistic LOINC code.
    ///
    /// # Returns
    ///
    /// A LOINC code (5-7 digit number)
    pub fn loinc(&mut self) -> String {
        let code = self.rng.random_range(10000..9999999);
        code.to_string()
    }

    /// Generate a realistic medication name.
    ///
    /// # Returns
    ///
    /// A common medication name
    pub fn medication(&mut self) -> String {
        let medications = &[
            "Atorvastatin",
            "Levothyroxine",
            "Lisinopril",
            "Metformin",
            "Amlodipine",
            "Metoprolol",
            "Omeprazole",
            "Simvastatin",
            "Losartan",
            "Albuterol",
        ];
        random_copy(self.rng, medications).unwrap_or("").to_string()
    }

    /// Generate a realistic allergen.
    ///
    /// # Returns
    ///
    /// A common allergen name
    pub fn allergen(&mut self) -> String {
        let allergens = &[
            "Penicillin",
            "Latex",
            "Peanuts",
            "Shellfish",
            "Eggs",
            "Milk",
            "Tree Nuts",
            "Soy",
            "Wheat",
            "Bee Stings",
        ];
        random_copy(self.rng, allergens).unwrap_or("").to_string()
    }

    /// Generate a realistic blood type.
    ///
    /// # Returns
    ///
    /// A blood type string (e.g., "A+", "O-")
    pub fn blood_type(&mut self) -> String {
        let blood_types = &["A+", "A-", "B+", "B-", "AB+", "AB-", "O+", "O-"];
        random_copy(self.rng, blood_types).unwrap_or("").to_string()
    }

    /// Generate a realistic ethnicity.
    ///
    /// # Returns
    ///
    /// An ethnicity string
    pub fn ethnicity(&mut self) -> String {
        let ethnicities = &[
            "Hispanic or Latino",
            "Not Hispanic or Latino",
            "Declined to Specify",
        ];
        random_copy(self.rng, ethnicities).unwrap_or("").to_string()
    }

    /// Generate a realistic race.
    ///
    /// # Returns
    ///
    /// A race string
    pub fn race(&mut self) -> String {
        let races = &[
            "American Indian or Alaska Native",
            "Asian",
            "Black or African American",
            "Native Hawaiian or Other Pacific Islander",
            "White",
            "Declined to Specify",
        ];
        random_copy(self.rng, races).unwrap_or("").to_string()
    }

    /// Generate a numeric string with the specified number of digits.
    ///
    /// # Arguments
    ///
    /// * `digits` - Number of digits in the output
    pub fn numeric(&mut self, digits: usize) -> String {
        let mut result = String::new();
        for _ in 0..digits {
            let digit = self.rng.random_range(0..10);
            result.push_str(&digit.to_string());
        }
        result
    }

    /// Generate a date within the specified range.
    ///
    /// # Arguments
    ///
    /// * `start` - Start date in YYYYMMDD format
    /// * `end` - End date in YYYYMMDD format
    ///
    /// # Returns
    ///
    /// A date string in YYYYMMDD format
    ///
    /// # Errors
    ///
    /// Returns [`DateError`] when either date cannot be parsed, the end date is
    /// before the start date, or the computed random date is out of range.
    pub fn date(&mut self, start: &str, end: &str) -> Result<String, DateError> {
        let start_date = chrono::NaiveDate::parse_from_str(start, "%Y%m%d")
            .map_err(|_err| DateError::InvalidDateFormat(start.to_string()))?;
        let end_date = chrono::NaiveDate::parse_from_str(end, "%Y%m%d")
            .map_err(|_err| DateError::InvalidDateFormat(end.to_string()))?;

        if end_date < start_date {
            return Err(DateError::InvalidDateRange {
                start: start.to_string(),
                end: end.to_string(),
            });
        }

        let duration = end_date.signed_duration_since(start_date);
        let days = duration.num_days();

        let random_days = self.rng.random_range(0..=days);
        let random_date = start_date
            .checked_add_signed(chrono::Duration::days(random_days))
            .ok_or(DateError::DateOutOfRange)?;

        Ok(random_date.format("%Y%m%d").to_string())
    }

    /// Generate a Gaussian distributed value.
    ///
    /// # Arguments
    ///
    /// * `mean` - Mean of the distribution
    /// * `sd` - Standard deviation
    /// * `precision` - Number of decimal places
    ///
    /// # Errors
    ///
    /// Returns [`GaussianError::InvalidParameters`] if the normal distribution
    /// cannot be constructed from the supplied mean and standard deviation.
    pub fn gaussian(
        &mut self,
        mean: f64,
        sd: f64,
        precision: usize,
    ) -> Result<String, GaussianError> {
        let normal = Normal::new(mean, sd).map_err(|_err| GaussianError::InvalidParameters)?;
        let value = self.rng.sample(normal);
        Ok(format!("{value:.precision$}"))
    }

    /// Generate a UUID v4.
    pub fn uuid_v4(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }

    /// Generate a current UTC timestamp.
    ///
    /// # Returns
    ///
    /// A timestamp in YYYYMMDDHHMMSS format
    pub fn dtm_now_utc(&self) -> String {
        let now = chrono::Utc::now();
        now.format("%Y%m%d%H%M%S").to_string()
    }

    /// Select a value from a list of options.
    ///
    /// # Arguments
    ///
    /// * `options` - Slice of string options
    pub fn select_from(&mut self, options: &[String]) -> Option<String> {
        if options.is_empty() {
            return None;
        }
        let index = self.rng.random_range(0..options.len());
        options.get(index).cloned()
    }

    /// Select a random value from a map.
    ///
    /// # Arguments
    ///
    /// * `map` - HashMap of key-value pairs
    pub fn select_from_map(
        &mut self,
        map: &std::collections::HashMap<String, String>,
    ) -> Option<String> {
        if map.is_empty() {
            return None;
        }
        let keys: Vec<&String> = map.keys().collect();
        let index = self.rng.random_range(0..keys.len());
        keys.get(index)
            .and_then(|random_key| map.get(*random_key))
            .cloned()
    }
}

/// Error type for date generation.
#[derive(Debug, Clone, PartialEq)]
pub enum DateError {
    /// Invalid date format (expected YYYYMMDD)
    InvalidDateFormat(String),
    /// End date is before start date.
    InvalidDateRange {
        /// Start date provided by the caller.
        start: String,
        /// End date provided by the caller.
        end: String,
    },
    /// Computed date exceeded the representable date range.
    DateOutOfRange,
}

impl std::fmt::Display for DateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DateError::InvalidDateFormat(s) => {
                write!(f, "Invalid date format: {s} (expected YYYYMMDD)")
            }
            DateError::InvalidDateRange { start, end } => {
                write!(f, "Invalid date range: {start} is after {end}")
            }
            DateError::DateOutOfRange => write!(f, "Generated date is out of range"),
        }
    }
}

impl std::error::Error for DateError {}

/// Error type for Gaussian generation.
#[derive(Debug, Clone, PartialEq)]
pub enum GaussianError {
    /// Invalid parameters (e.g., negative standard deviation)
    InvalidParameters,
}

impl std::fmt::Display for GaussianError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GaussianError::InvalidParameters => write!(f, "Invalid Gaussian parameters"),
        }
    }
}

impl std::error::Error for GaussianError {}

/// Value source for generating realistic test data.
///
/// This enum represents different types of data that can be generated.
/// It can be used in templates or configuration to specify what kind
/// of test data to generate.
#[derive(Debug, Clone, PartialEq)]
pub enum FakerValue {
    /// Fixed string value
    Fixed(String),
    /// Select from a list of options
    From(Vec<String>),
    /// Numeric string with specified digits
    Numeric {
        /// Number of digits to emit.
        digits: usize,
    },
    /// Date within a range (YYYYMMDD format)
    Date {
        /// Start date (inclusive), `YYYYMMDD`.
        start: String,
        /// End date (inclusive), `YYYYMMDD`.
        end: String,
    },
    /// Gaussian distributed numeric value
    Gaussian {
        /// Mean value.
        mean: f64,
        /// Standard deviation.
        sd: f64,
        /// Number of decimal places in output.
        precision: usize,
    },
    /// Select from a key-value map
    Map(std::collections::HashMap<String, String>),
    /// UUID v4
    UuidV4,
    /// Current UTC timestamp
    DtmNowUtc,
    /// Realistic name with optional gender
    RealisticName {
        /// Optional gender hint passed to the name generator.
        gender: Option<String>,
    },
    /// Realistic address
    RealisticAddress,
    /// Realistic phone number
    RealisticPhone,
    /// Realistic SSN
    RealisticSsn,
    /// Realistic Medical Record Number
    RealisticMrn,
    /// Realistic ICD-10 code
    RealisticIcd10,
    /// Realistic LOINC code
    RealisticLoinc,
    /// Realistic medication name
    RealisticMedication,
    /// Realistic allergen
    RealisticAllergen,
    /// Realistic blood type
    RealisticBloodType,
    /// Realistic ethnicity
    RealisticEthnicity,
    /// Realistic race
    RealisticRace,
}

impl FakerValue {
    /// Generate a value using the given faker instance.
    ///
    /// # Arguments
    ///
    /// * `faker` - Faker instance to use for generation
    ///
    /// # Returns
    ///
    /// The generated string value, or an error message if generation failed.
    ///
    /// # Errors
    ///
    /// Returns [`GenerateError`] when an option/map source is empty or when a
    /// date/Gaussian source cannot be generated.
    pub fn generate<R: Rng>(&self, faker: &mut Faker<R>) -> Result<String, GenerateError> {
        match self {
            FakerValue::Fixed(value) => Ok(value.clone()),
            FakerValue::From(options) => faker
                .select_from(options)
                .ok_or(GenerateError::EmptyOptions),
            FakerValue::Numeric { digits } => Ok(faker.numeric(*digits)),
            FakerValue::Date { start, end } => faker.date(start, end).map_err(GenerateError::Date),
            FakerValue::Gaussian {
                mean,
                sd,
                precision,
            } => faker
                .gaussian(*mean, *sd, *precision)
                .map_err(GenerateError::Gaussian),
            FakerValue::Map(mapping) => faker
                .select_from_map(mapping)
                .ok_or(GenerateError::EmptyMap),
            FakerValue::UuidV4 => Ok(faker.uuid_v4()),
            FakerValue::DtmNowUtc => Ok(faker.dtm_now_utc()),
            FakerValue::RealisticName { gender } => Ok(faker.name(gender.as_deref())),
            FakerValue::RealisticAddress => Ok(faker.address()),
            FakerValue::RealisticPhone => Ok(faker.phone()),
            FakerValue::RealisticSsn => Ok(faker.ssn()),
            FakerValue::RealisticMrn => Ok(faker.mrn()),
            FakerValue::RealisticIcd10 => Ok(faker.icd10()),
            FakerValue::RealisticLoinc => Ok(faker.loinc()),
            FakerValue::RealisticMedication => Ok(faker.medication()),
            FakerValue::RealisticAllergen => Ok(faker.allergen()),
            FakerValue::RealisticBloodType => Ok(faker.blood_type()),
            FakerValue::RealisticEthnicity => Ok(faker.ethnicity()),
            FakerValue::RealisticRace => Ok(faker.race()),
        }
    }
}

/// Error type for value generation.
#[derive(Debug, Clone, PartialEq)]
pub enum GenerateError {
    /// Empty options list
    EmptyOptions,
    /// Empty map
    EmptyMap,
    /// Date generation error
    Date(DateError),
    /// Gaussian generation error
    Gaussian(GaussianError),
}

impl std::fmt::Display for GenerateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GenerateError::EmptyOptions => write!(f, "Cannot select from empty options"),
            GenerateError::EmptyMap => write!(f, "Cannot select from empty map"),
            GenerateError::Date(e) => write!(f, "Date generation error: {e}"),
            GenerateError::Gaussian(e) => write!(f, "Gaussian generation error: {e}"),
        }
    }
}

impl std::error::Error for GenerateError {}

// Re-export rand types for convenience
pub use rand::Rng;
pub use rand::SeedableRng;
pub use rand::rngs::StdRng;
