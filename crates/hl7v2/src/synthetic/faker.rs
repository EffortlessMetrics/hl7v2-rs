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

#[cfg(test)]
mod tests {
    #![expect(
        clippy::panic,
        reason = "Faker unit tests fail explicitly on test setup errors."
    )]

    use super::{DateError, Faker, FakerValue, GaussianError, GenerateError};
    use rand::SeedableRng;
    use rand::rngs::StdRng;
    use std::collections::HashMap;

    fn seeded_rng() -> StdRng {
        StdRng::seed_from_u64(42)
    }

    #[test]
    fn name_male_returns_last_caret_first() {
        let mut rng = seeded_rng();
        let mut faker = Faker::new(&mut rng);
        let name = faker.name(Some("M"));
        let parts: Vec<&str> = name.split('^').collect();
        assert_eq!(parts.len(), 2, "expected LAST^FIRST format, got {name}");
        let male_pool = [
            "James", "John", "Robert", "Michael", "William", "David", "Richard", "Joseph",
            "Thomas", "Charles",
        ];
        let first = parts.get(1).copied().unwrap_or("");
        assert!(
            male_pool.contains(&first),
            "first name {first} should come from the male pool"
        );
    }

    #[test]
    fn name_female_uses_female_pool() {
        let mut rng = seeded_rng();
        let mut faker = Faker::new(&mut rng);
        let name = faker.name(Some("F"));
        let parts: Vec<&str> = name.split('^').collect();
        assert_eq!(parts.len(), 2, "expected LAST^FIRST format, got {name}");
        let female_pool = [
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
        ];
        let first = parts.get(1).copied().unwrap_or("");
        assert!(
            female_pool.contains(&first),
            "first name {first} should come from the female pool"
        );
    }

    #[test]
    fn name_unknown_gender_falls_back_to_mixed_pool() {
        let mut rng = seeded_rng();
        let mut faker = Faker::new(&mut rng);
        // "U" and None and any other value all map to the mixed pool.
        let name_u = faker.name(Some("U"));
        let name_none = faker.name(None);
        let name_other = faker.name(Some("X"));
        for sample in [&name_u, &name_none, &name_other] {
            assert!(
                sample.contains('^'),
                "name {sample} should contain caret separator"
            );
            assert_eq!(
                sample.split('^').count(),
                2,
                "expected LAST^FIRST in {sample}"
            );
            assert!(!sample.starts_with('^'), "last name must be non-empty");
            assert!(!sample.ends_with('^'), "first name must be non-empty");
        }
    }

    #[test]
    fn name_with_fixed_seed_is_deterministic() {
        let mut rng_a = StdRng::seed_from_u64(123);
        let mut rng_b = StdRng::seed_from_u64(123);
        let mut faker_a = Faker::new(&mut rng_a);
        let mut faker_b = Faker::new(&mut rng_b);
        assert_eq!(faker_a.name(Some("M")), faker_b.name(Some("M")));
    }

    #[test]
    fn address_has_expected_caret_structure() {
        let mut rng = seeded_rng();
        let mut faker = Faker::new(&mut rng);
        let addr = faker.address();
        // Format is "NUMBER STREET^^CITY^STATE^ZIP^USA"
        let parts: Vec<&str> = addr.split('^').collect();
        assert_eq!(parts.len(), 6, "expected 6 caret-separated parts in {addr}");
        assert_eq!(parts.get(5).copied(), Some("USA"));
        // ZIP is 5 digits
        let zip = parts.get(4).copied().unwrap_or("");
        assert_eq!(zip.len(), 5, "zip {zip} should be 5 digits");
        assert!(
            zip.chars().all(|c| c.is_ascii_digit()),
            "zip should be all digits"
        );
        // State is two letters
        let state = parts.get(3).copied().unwrap_or("");
        assert_eq!(state.len(), 2, "state {state} should be 2 letters");
    }

    #[test]
    fn phone_has_parens_and_dash() {
        let mut rng = seeded_rng();
        let mut faker = Faker::new(&mut rng);
        let phone = faker.phone();
        assert!(phone.starts_with('('), "phone {phone} should start with (");
        assert!(phone.contains(')'), "phone {phone} should contain )");
        assert!(phone.contains('-'), "phone {phone} should contain -");
        // Length should be at least "(200)200-1000" = 13 chars
        assert!(phone.len() >= 13, "phone {phone} too short");
    }

    #[test]
    fn ssn_has_three_dashed_groups() {
        let mut rng = seeded_rng();
        let mut faker = Faker::new(&mut rng);
        let ssn = faker.ssn();
        let parts: Vec<&str> = ssn.split('-').collect();
        assert_eq!(parts.len(), 3, "ssn {ssn} should have 3 groups");
        assert_eq!(parts.first().map(|s| s.len()), Some(3));
        assert_eq!(parts.get(1).map(|s| s.len()), Some(2));
        assert_eq!(parts.get(2).map(|s| s.len()), Some(4));
        for group in &parts {
            assert!(
                group.chars().all(|c| c.is_ascii_digit()),
                "ssn group {group} should be digits"
            );
        }
    }

    #[test]
    fn mrn_has_six_to_ten_digits() {
        let mut rng = seeded_rng();
        let mut faker = Faker::new(&mut rng);
        for _ in 0..20 {
            let mrn = faker.mrn();
            assert!(
                (6..=10).contains(&mrn.len()),
                "mrn {mrn} length {} not in 6..=10",
                mrn.len()
            );
            assert!(
                mrn.chars().all(|c| c.is_ascii_digit()),
                "mrn {mrn} should be all digits"
            );
        }
    }

    #[test]
    fn date_within_range_returns_yyyymmdd() {
        let mut rng = seeded_rng();
        let mut faker = Faker::new(&mut rng);
        let Ok(date) = faker.date("20200101", "20201231") else {
            panic!("date generation should succeed");
        };
        assert_eq!(date.len(), 8, "date {date} should be YYYYMMDD");
        assert!(date.chars().all(|c| c.is_ascii_digit()));
        assert!(date.as_str() >= "20200101");
        assert!(date.as_str() <= "20201231");
    }

    #[test]
    fn date_single_day_range_returns_that_day() {
        let mut rng = seeded_rng();
        let mut faker = Faker::new(&mut rng);
        let Ok(date) = faker.date("20230615", "20230615") else {
            panic!("single-day date should succeed");
        };
        assert_eq!(date, "20230615");
    }

    #[test]
    fn date_invalid_start_format_returns_error() {
        let mut rng = seeded_rng();
        let mut faker = Faker::new(&mut rng);
        let result = faker.date("not-a-date", "20201231");
        assert!(matches!(
            result,
            Err(DateError::InvalidDateFormat(ref s)) if s == "not-a-date"
        ));
    }

    #[test]
    fn date_invalid_end_format_returns_error() {
        let mut rng = seeded_rng();
        let mut faker = Faker::new(&mut rng);
        let result = faker.date("20200101", "garbage");
        assert!(matches!(
            result,
            Err(DateError::InvalidDateFormat(ref s)) if s == "garbage"
        ));
    }

    #[test]
    fn date_end_before_start_returns_range_error() {
        let mut rng = seeded_rng();
        let mut faker = Faker::new(&mut rng);
        let result = faker.date("20201231", "20200101");
        assert!(matches!(result, Err(DateError::InvalidDateRange { .. })));
    }

    #[test]
    fn gaussian_returns_formatted_value() {
        let mut rng = seeded_rng();
        let mut faker = Faker::new(&mut rng);
        let Ok(value) = faker.gaussian(100.0, 15.0, 2) else {
            panic!("gaussian generation should succeed");
        };
        // Should have exactly 2 decimal places
        let parts: Vec<&str> = value.split('.').collect();
        assert_eq!(parts.len(), 2, "gaussian {value} should have a decimal");
        assert_eq!(
            parts.get(1).map(|s| s.len()),
            Some(2),
            "expected 2 decimal places in {value}"
        );
    }

    #[test]
    fn gaussian_precision_zero_emits_no_decimal_point() {
        let mut rng = seeded_rng();
        let mut faker = Faker::new(&mut rng);
        let Ok(value) = faker.gaussian(50.0, 5.0, 0) else {
            panic!("gaussian generation should succeed");
        };
        assert!(
            !value.contains('.'),
            "precision-0 result {value} should not contain a decimal point"
        );
    }

    #[test]
    fn gaussian_infinite_sd_returns_error() {
        let mut rng = seeded_rng();
        let mut faker = Faker::new(&mut rng);
        // Non-finite SD is invalid in rand_distr::Normal.
        let result = faker.gaussian(0.0, f64::INFINITY, 2);
        assert_eq!(result, Err(GaussianError::InvalidParameters));
    }

    #[test]
    fn gaussian_nan_sd_returns_error() {
        let mut rng = seeded_rng();
        let mut faker = Faker::new(&mut rng);
        let result = faker.gaussian(0.0, f64::NAN, 2);
        assert_eq!(result, Err(GaussianError::InvalidParameters));
    }

    #[test]
    fn select_from_empty_returns_none() {
        let mut rng = seeded_rng();
        let mut faker = Faker::new(&mut rng);
        let empty: Vec<String> = Vec::new();
        assert_eq!(faker.select_from(&empty), None);
    }

    #[test]
    fn select_from_picks_from_options() {
        let mut rng = seeded_rng();
        let mut faker = Faker::new(&mut rng);
        let options = vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()];
        for _ in 0..10 {
            let picked = faker.select_from(&options);
            let Some(value) = picked else {
                panic!("select_from should return Some for non-empty options");
            };
            assert!(options.contains(&value), "value {value} not in options");
        }
    }

    #[test]
    fn select_from_map_empty_returns_none() {
        let mut rng = seeded_rng();
        let mut faker = Faker::new(&mut rng);
        let empty: HashMap<String, String> = HashMap::new();
        assert_eq!(faker.select_from_map(&empty), None);
    }

    #[test]
    fn select_from_map_returns_a_value_from_map() {
        let mut rng = seeded_rng();
        let mut faker = Faker::new(&mut rng);
        let mut map: HashMap<String, String> = HashMap::new();
        map.insert("k1".to_string(), "v1".to_string());
        map.insert("k2".to_string(), "v2".to_string());
        map.insert("k3".to_string(), "v3".to_string());
        let values: Vec<String> = map.values().cloned().collect();
        for _ in 0..10 {
            let Some(picked) = faker.select_from_map(&map) else {
                panic!("select_from_map should return Some for non-empty map");
            };
            assert!(values.contains(&picked), "{picked} not a map value");
        }
    }

    #[test]
    fn icd10_loinc_medication_allergen_blood_ethnicity_race_are_non_empty() {
        let mut rng = seeded_rng();
        let mut faker = Faker::new(&mut rng);
        assert!(faker.icd10().contains('.'));
        let loinc = faker.loinc();
        assert!(loinc.chars().all(|c| c.is_ascii_digit()));
        assert!(!faker.medication().is_empty());
        assert!(!faker.allergen().is_empty());
        assert!(!faker.blood_type().is_empty());
        assert!(!faker.ethnicity().is_empty());
        assert!(!faker.race().is_empty());
    }

    #[test]
    fn numeric_emits_exact_digit_count() {
        let mut rng = seeded_rng();
        let mut faker = Faker::new(&mut rng);
        let value = faker.numeric(7);
        assert_eq!(value.len(), 7);
        assert!(value.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn numeric_zero_digits_returns_empty() {
        let mut rng = seeded_rng();
        let mut faker = Faker::new(&mut rng);
        assert_eq!(faker.numeric(0), "");
    }

    #[test]
    fn uuid_v4_is_well_formed() {
        let mut rng = seeded_rng();
        let faker = Faker::new(&mut rng);
        let uuid = faker.uuid_v4();
        // Standard UUID string length is 36 with dashes.
        assert_eq!(uuid.len(), 36, "uuid {uuid} should be 36 chars");
        assert_eq!(uuid.matches('-').count(), 4);
    }

    #[test]
    fn dtm_now_utc_is_yyyymmddhhmmss() {
        let mut rng = seeded_rng();
        let faker = Faker::new(&mut rng);
        let dtm = faker.dtm_now_utc();
        assert_eq!(dtm.len(), 14, "dtm {dtm} should be 14 chars");
        assert!(dtm.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn faker_value_fixed_round_trips_string() {
        let mut rng = seeded_rng();
        let mut faker = Faker::new(&mut rng);
        let fv = FakerValue::Fixed("hello".to_string());
        assert_eq!(fv.generate(&mut faker).ok(), Some("hello".to_string()));
    }

    #[test]
    fn faker_value_from_empty_returns_error() {
        let mut rng = seeded_rng();
        let mut faker = Faker::new(&mut rng);
        let fv = FakerValue::From(Vec::new());
        assert_eq!(fv.generate(&mut faker), Err(GenerateError::EmptyOptions));
    }

    #[test]
    fn faker_value_map_empty_returns_error() {
        let mut rng = seeded_rng();
        let mut faker = Faker::new(&mut rng);
        let fv = FakerValue::Map(HashMap::new());
        assert_eq!(fv.generate(&mut faker), Err(GenerateError::EmptyMap));
    }

    #[test]
    fn date_error_display_includes_context() {
        let err = DateError::InvalidDateFormat("foo".to_string());
        let s = format!("{err}");
        assert!(s.contains("foo"));

        let err = DateError::InvalidDateRange {
            start: "20210101".to_string(),
            end: "20200101".to_string(),
        };
        let s = format!("{err}");
        assert!(s.contains("20210101"));
        assert!(s.contains("20200101"));

        let err = DateError::DateOutOfRange;
        let s = format!("{err}");
        assert!(s.contains("out of range"));
    }

    #[test]
    fn gaussian_error_display_is_human_readable() {
        let err = GaussianError::InvalidParameters;
        let s = format!("{err}");
        assert!(s.contains("Gaussian"));
    }

    #[test]
    fn generate_error_display_wraps_inner_errors() {
        let err = GenerateError::Date(DateError::DateOutOfRange);
        let s = format!("{err}");
        assert!(s.contains("Date generation error"));

        let err = GenerateError::Gaussian(GaussianError::InvalidParameters);
        let s = format!("{err}");
        assert!(s.contains("Gaussian generation error"));

        let err = GenerateError::EmptyOptions;
        let s = format!("{err}");
        assert!(s.contains("empty options"));

        let err = GenerateError::EmptyMap;
        let s = format!("{err}");
        assert!(s.contains("empty map"));
    }
}
