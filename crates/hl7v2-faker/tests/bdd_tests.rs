//! BDD tests for hl7v2-faker using Cucumber
//!
//! Run with: cargo test --test bdd_tests -p hl7v2-faker

use std::collections::HashMap;

use cucumber::{World, given, then, when};
use hl7v2_faker::{Faker, FakerValue, GenerateError, StdRng};
use rand::SeedableRng;

/// Test world for faker BDD tests
#[derive(Debug, World)]
#[world(init = Self::new)]
pub struct FakerWorld {
    /// The seeded RNG
    rng: StdRng,
    /// Generated name result
    name: Option<String>,
    /// Second name for comparison
    name2: Option<String>,
    /// Generated address result
    address: Option<String>,
    /// Generated phone result
    phone: Option<String>,
    /// Generated SSN result
    ssn: Option<String>,
    /// Generated MRN result
    mrn: Option<String>,
    /// Generated ICD-10 code result
    icd10: Option<String>,
    /// Generated LOINC code result
    loinc: Option<String>,
    /// Generated medication result
    medication: Option<String>,
    /// Generated allergen result
    allergen: Option<String>,
    /// Generated blood type result
    blood_type: Option<String>,
    /// Generated ethnicity result
    ethnicity: Option<String>,
    /// Generated race result
    race: Option<String>,
    /// Generated numeric string result
    numeric_string: Option<String>,
    /// Generated date result
    date_result: Option<Result<String, hl7v2_faker::DateError>>,
    /// Generated Gaussian value result
    gaussian_value: Option<String>,
    /// Generated UUID result
    uuid: Option<String>,
    /// Generated timestamp result
    timestamp: Option<String>,
    /// Selected value from list
    selected_value: Option<Option<String>>,
    /// Selected value from map
    map_selected_value: Option<Option<String>>,
    /// Generated FakerValue result
    faker_value_result: Option<Result<String, GenerateError>>,
    /// First set of all data types for determinism test
    all_data_set1: Option<Vec<String>>,
    /// Second set of all data types for determinism test
    all_data_set2: Option<Vec<String>>,
    /// Gaussian mean for validation
    gaussian_mean: f64,
    /// Gaussian stddev for validation
    gaussian_sd: f64,
}

impl FakerWorld {
    fn new() -> Self {
        Self {
            rng: StdRng::seed_from_u64(0),
            name: None,
            name2: None,
            address: None,
            phone: None,
            ssn: None,
            mrn: None,
            icd10: None,
            loinc: None,
            medication: None,
            allergen: None,
            blood_type: None,
            ethnicity: None,
            race: None,
            numeric_string: None,
            date_result: None,
            gaussian_value: None,
            uuid: None,
            timestamp: None,
            selected_value: None,
            map_selected_value: None,
            faker_value_result: None,
            all_data_set1: None,
            all_data_set2: None,
            gaussian_mean: 0.0,
            gaussian_sd: 0.0,
        }
    }
}

/// Generate all deterministic data types and return as a Vec<String>.
fn generate_all_data(rng: &mut StdRng) -> Vec<String> {
    let mut faker = Faker::new(rng);
    vec![
        faker.name(Some("M")),
        faker.name(Some("F")),
        faker.name(None),
        faker.address(),
        faker.phone(),
        faker.ssn(),
        faker.mrn(),
        faker.icd10(),
        faker.loinc(),
        faker.medication(),
        faker.allergen(),
        faker.blood_type(),
        faker.ethnicity(),
        faker.race(),
        faker.numeric(8),
        faker.date("20200101", "20201231").unwrap(),
        faker.gaussian(100.0, 10.0, 2).unwrap(),
    ]
}

// ============================================================================
// Given Steps
// ============================================================================

#[given(regex = r"^a faker with seed (\d+)$")]
fn given_faker_with_seed(world: &mut FakerWorld, seed: u64) {
    world.rng = StdRng::seed_from_u64(seed);
}

// ============================================================================
// When Steps — Name Generation
// ============================================================================

#[when(regex = r#"^I generate a name with gender "([^"]*)"$"#)]
fn when_generate_name_with_gender(world: &mut FakerWorld, gender: String) {
    let mut faker = Faker::new(&mut world.rng);
    world.name = Some(faker.name(Some(&gender)));
}

#[when("I generate a name with no gender")]
fn when_generate_name_no_gender(world: &mut FakerWorld) {
    let mut faker = Faker::new(&mut world.rng);
    world.name = Some(faker.name(None));
}

#[when(regex = r#"^I generate another name with gender "([^"]*)" using seed (\d+)$"#)]
fn when_generate_another_name(world: &mut FakerWorld, gender: String, seed: u64) {
    let mut rng2 = StdRng::seed_from_u64(seed);
    let mut faker2 = Faker::new(&mut rng2);
    world.name2 = Some(faker2.name(Some(&gender)));
}

// ============================================================================
// When Steps — Address Generation
// ============================================================================

#[when("I generate an address")]
fn when_generate_address(world: &mut FakerWorld) {
    let mut faker = Faker::new(&mut world.rng);
    world.address = Some(faker.address());
}

// ============================================================================
// When Steps — Phone Generation
// ============================================================================

#[when("I generate a phone number")]
fn when_generate_phone(world: &mut FakerWorld) {
    let mut faker = Faker::new(&mut world.rng);
    world.phone = Some(faker.phone());
}

// ============================================================================
// When Steps — SSN Generation
// ============================================================================

#[when("I generate an SSN")]
fn when_generate_ssn(world: &mut FakerWorld) {
    let mut faker = Faker::new(&mut world.rng);
    world.ssn = Some(faker.ssn());
}

// ============================================================================
// When Steps — MRN Generation
// ============================================================================

#[when("I generate an MRN")]
fn when_generate_mrn(world: &mut FakerWorld) {
    let mut faker = Faker::new(&mut world.rng);
    world.mrn = Some(faker.mrn());
}

// ============================================================================
// When Steps — Medical Code Generation
// ============================================================================

#[when("I generate an ICD-10 code")]
fn when_generate_icd10(world: &mut FakerWorld) {
    let mut faker = Faker::new(&mut world.rng);
    world.icd10 = Some(faker.icd10());
}

#[when("I generate a LOINC code")]
fn when_generate_loinc(world: &mut FakerWorld) {
    let mut faker = Faker::new(&mut world.rng);
    world.loinc = Some(faker.loinc());
}

// ============================================================================
// When Steps — Medication and Allergen Generation
// ============================================================================

#[when("I generate a medication name")]
fn when_generate_medication(world: &mut FakerWorld) {
    let mut faker = Faker::new(&mut world.rng);
    world.medication = Some(faker.medication());
}

#[when("I generate an allergen name")]
fn when_generate_allergen(world: &mut FakerWorld) {
    let mut faker = Faker::new(&mut world.rng);
    world.allergen = Some(faker.allergen());
}

// ============================================================================
// When Steps — Patient Demographics
// ============================================================================

#[when("I generate a blood type")]
fn when_generate_blood_type(world: &mut FakerWorld) {
    let mut faker = Faker::new(&mut world.rng);
    world.blood_type = Some(faker.blood_type());
}

#[when("I generate an ethnicity")]
fn when_generate_ethnicity(world: &mut FakerWorld) {
    let mut faker = Faker::new(&mut world.rng);
    world.ethnicity = Some(faker.ethnicity());
}

#[when("I generate a race")]
fn when_generate_race(world: &mut FakerWorld) {
    let mut faker = Faker::new(&mut world.rng);
    world.race = Some(faker.race());
}

// ============================================================================
// When Steps — Numeric Generation
// ============================================================================

#[when(regex = r"^I generate a numeric string of (\d+) digits$")]
fn when_generate_numeric(world: &mut FakerWorld, digits: usize) {
    let mut faker = Faker::new(&mut world.rng);
    world.numeric_string = Some(faker.numeric(digits));
}

// ============================================================================
// When Steps — Date Generation
// ============================================================================

#[when(regex = r#"^I generate a date between "([^"]*)" and "([^"]*)"$"#)]
fn when_generate_date(world: &mut FakerWorld, start: String, end: String) {
    let mut faker = Faker::new(&mut world.rng);
    world.date_result = Some(faker.date(&start, &end));
}

// ============================================================================
// When Steps — Gaussian Generation
// ============================================================================

#[when(regex = r"^I generate a Gaussian value with mean ([^ ]+) stddev ([^ ]+) precision (\d+)$")]
fn when_generate_gaussian(world: &mut FakerWorld, mean: f64, sd: f64, precision: usize) {
    world.gaussian_mean = mean;
    world.gaussian_sd = sd;
    let mut faker = Faker::new(&mut world.rng);
    world.gaussian_value = Some(faker.gaussian(mean, sd, precision).unwrap());
}

// ============================================================================
// When Steps — UUID Generation
// ============================================================================

#[when("I generate a UUID v4")]
fn when_generate_uuid(world: &mut FakerWorld) {
    let faker = Faker::new(&mut world.rng);
    world.uuid = Some(faker.uuid_v4());
}

// ============================================================================
// When Steps — Timestamp Generation
// ============================================================================

#[when("I generate a UTC timestamp")]
fn when_generate_timestamp(world: &mut FakerWorld) {
    let faker = Faker::new(&mut world.rng);
    world.timestamp = Some(faker.dtm_now_utc());
}

// ============================================================================
// When Steps — Selection Functions
// ============================================================================

#[when(regex = r#"^I select from the list "([^"]*)"$"#)]
fn when_select_from_list(world: &mut FakerWorld, items: String) {
    let options: Vec<String> = items.split(',').map(|s| s.to_string()).collect();
    let mut faker = Faker::new(&mut world.rng);
    world.selected_value = Some(faker.select_from(&options));
}

#[when("I select from an empty list")]
fn when_select_from_empty_list(world: &mut FakerWorld) {
    let options: Vec<String> = vec![];
    let mut faker = Faker::new(&mut world.rng);
    world.selected_value = Some(faker.select_from(&options));
}

#[when(regex = r#"^I select from a map with entries "([^"]*)"$"#)]
fn when_select_from_map(world: &mut FakerWorld, entries: String) {
    let mut map = HashMap::new();
    for entry in entries.split(',') {
        let parts: Vec<&str> = entry.split('=').collect();
        map.insert(parts[0].to_string(), parts[1].to_string());
    }
    let mut faker = Faker::new(&mut world.rng);
    world.map_selected_value = Some(faker.select_from_map(&map));
}

#[when("I select from an empty map")]
fn when_select_from_empty_map(world: &mut FakerWorld) {
    let map: HashMap<String, String> = HashMap::new();
    let mut faker = Faker::new(&mut world.rng);
    world.map_selected_value = Some(faker.select_from_map(&map));
}

// ============================================================================
// When Steps — FakerValue Enum Generation
// ============================================================================

#[when(regex = r#"^I generate a FakerValue::Fixed with value "([^"]*)"$"#)]
fn when_faker_value_fixed(world: &mut FakerWorld, value: String) {
    let fv = FakerValue::Fixed(value);
    let mut faker = Faker::new(&mut world.rng);
    world.faker_value_result = Some(fv.generate(&mut faker));
}

#[when(regex = r#"^I generate a FakerValue::From with options "([^"]*)"$"#)]
fn when_faker_value_from(world: &mut FakerWorld, options: String) {
    let opts: Vec<String> = options.split(',').map(|s| s.to_string()).collect();
    let fv = FakerValue::From(opts);
    let mut faker = Faker::new(&mut world.rng);
    world.faker_value_result = Some(fv.generate(&mut faker));
}

#[when("I generate a FakerValue::From with no options")]
fn when_faker_value_from_empty(world: &mut FakerWorld) {
    let fv = FakerValue::From(vec![]);
    let mut faker = Faker::new(&mut world.rng);
    world.faker_value_result = Some(fv.generate(&mut faker));
}

#[when(regex = r"^I generate a FakerValue::Numeric with (\d+) digits$")]
fn when_faker_value_numeric(world: &mut FakerWorld, digits: usize) {
    let fv = FakerValue::Numeric { digits };
    let mut faker = Faker::new(&mut world.rng);
    world.faker_value_result = Some(fv.generate(&mut faker));
}

#[when(regex = r#"^I generate a FakerValue::RealisticName for gender "([^"]*)"$"#)]
fn when_faker_value_realistic_name(world: &mut FakerWorld, gender: String) {
    let fv = FakerValue::RealisticName {
        gender: Some(gender),
    };
    let mut faker = Faker::new(&mut world.rng);
    world.faker_value_result = Some(fv.generate(&mut faker));
}

#[when("I generate a FakerValue::RealisticAddress")]
fn when_faker_value_realistic_address(world: &mut FakerWorld) {
    let fv = FakerValue::RealisticAddress;
    let mut faker = Faker::new(&mut world.rng);
    world.faker_value_result = Some(fv.generate(&mut faker));
}

#[when("I generate a FakerValue::RealisticPhone")]
fn when_faker_value_realistic_phone(world: &mut FakerWorld) {
    let fv = FakerValue::RealisticPhone;
    let mut faker = Faker::new(&mut world.rng);
    world.faker_value_result = Some(fv.generate(&mut faker));
}

#[when("I generate a FakerValue::RealisticSsn")]
fn when_faker_value_realistic_ssn(world: &mut FakerWorld) {
    let fv = FakerValue::RealisticSsn;
    let mut faker = Faker::new(&mut world.rng);
    world.faker_value_result = Some(fv.generate(&mut faker));
}

#[when("I generate a FakerValue::RealisticMrn")]
fn when_faker_value_realistic_mrn(world: &mut FakerWorld) {
    let fv = FakerValue::RealisticMrn;
    let mut faker = Faker::new(&mut world.rng);
    world.faker_value_result = Some(fv.generate(&mut faker));
}

#[when("I generate a FakerValue::RealisticBloodType")]
fn when_faker_value_realistic_blood_type(world: &mut FakerWorld) {
    let fv = FakerValue::RealisticBloodType;
    let mut faker = Faker::new(&mut world.rng);
    world.faker_value_result = Some(fv.generate(&mut faker));
}

#[when("I generate a FakerValue::Map with no entries")]
fn when_faker_value_map_empty(world: &mut FakerWorld) {
    let fv = FakerValue::Map(HashMap::new());
    let mut faker = Faker::new(&mut world.rng);
    world.faker_value_result = Some(fv.generate(&mut faker));
}

// ============================================================================
// When Steps — Determinism
// ============================================================================

#[when("I generate all data types")]
fn when_generate_all_data(world: &mut FakerWorld) {
    world.all_data_set1 = Some(generate_all_data(&mut world.rng));
}

#[when(regex = r"^I generate all data types again with seed (\d+)$")]
fn when_generate_all_data_again(world: &mut FakerWorld, seed: u64) {
    let mut rng2 = StdRng::seed_from_u64(seed);
    world.all_data_set2 = Some(generate_all_data(&mut rng2));
}

// ============================================================================
// Then Steps — Name Generation
// ============================================================================

#[then(regex = r#"^the name should be in "LAST\^FIRST" format$"#)]
fn then_name_in_hl7_format(world: &mut FakerWorld) {
    let name = world.name.as_ref().expect("No name was generated");
    assert!(name.contains('^'), "Name '{}' does not contain '^'", name);
    let parts: Vec<&str> = name.split('^').collect();
    assert_eq!(
        parts.len(),
        2,
        "Name '{}' is not in LAST^FIRST format",
        name
    );
    assert!(!parts[0].is_empty(), "Last name is empty");
    assert!(!parts[1].is_empty(), "First name is empty");
}

#[then("the first name should be a known male name")]
fn then_first_name_is_male(world: &mut FakerWorld) {
    let name = world.name.as_ref().expect("No name was generated");
    let first = name.split('^').nth(1).expect("No first name in result");
    let male_names = [
        "James", "John", "Robert", "Michael", "William", "David", "Richard", "Joseph", "Thomas",
        "Charles",
    ];
    assert!(
        male_names.contains(&first),
        "First name '{}' is not a known male name",
        first
    );
}

#[then("the first name should be a known female name")]
fn then_first_name_is_female(world: &mut FakerWorld) {
    let name = world.name.as_ref().expect("No name was generated");
    let first = name.split('^').nth(1).expect("No first name in result");
    let female_names = [
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
    assert!(
        female_names.contains(&first),
        "First name '{}' is not a known female name",
        first
    );
}

#[then("both names should be identical")]
fn then_names_identical(world: &mut FakerWorld) {
    let name1 = world.name.as_ref().expect("No first name was generated");
    let name2 = world.name2.as_ref().expect("No second name was generated");
    assert_eq!(name1, name2, "Names differ: '{}' vs '{}'", name1, name2);
}

// ============================================================================
// Then Steps — Address Generation
// ============================================================================

#[then("the address should contain component separators")]
fn then_address_has_separators(world: &mut FakerWorld) {
    let address = world.address.as_ref().expect("No address was generated");
    assert!(
        address.contains('^'),
        "Address '{}' has no component separators",
        address
    );
}

#[then(regex = r#"^the address should end with "([^"]*)"$"#)]
fn then_address_ends_with(world: &mut FakerWorld, suffix: String) {
    let address = world.address.as_ref().expect("No address was generated");
    assert!(
        address.ends_with(&suffix),
        "Address '{}' does not end with '{}'",
        address,
        suffix
    );
}

#[then(regex = r"^the address should have a street number between (\d+) and (\d+)$")]
fn then_address_street_number_range(world: &mut FakerWorld, min: i32, max: i32) {
    let address = world.address.as_ref().expect("No address was generated");
    let street_number: String = address.chars().take_while(|c| c.is_ascii_digit()).collect();
    let num: i32 = street_number
        .parse()
        .expect("Could not parse street number");
    assert!(
        (min..=max).contains(&num),
        "Street number {} is not between {} and {}",
        num,
        min,
        max
    );
}

// ============================================================================
// Then Steps — Phone Generation
// ============================================================================

#[then(regex = r#"^the phone number should match the format "\(XXX\)XXX-XXXX"$"#)]
fn then_phone_format(world: &mut FakerWorld) {
    let phone = world.phone.as_ref().expect("No phone was generated");
    assert!(
        phone.starts_with('('),
        "Phone '{}' doesn't start with '('",
        phone
    );
    assert!(phone.contains(')'), "Phone '{}' doesn't contain ')'", phone);
    assert!(phone.contains('-'), "Phone '{}' doesn't contain '-'", phone);
}

#[then(regex = r"^the phone number should be (\d+) characters long$")]
fn then_phone_length(world: &mut FakerWorld, length: usize) {
    let phone = world.phone.as_ref().expect("No phone was generated");
    assert_eq!(
        phone.len(),
        length,
        "Phone '{}' has length {} but expected {}",
        phone,
        phone.len(),
        length
    );
}

// ============================================================================
// Then Steps — SSN Generation
// ============================================================================

#[then(regex = r#"^the SSN should match the format "XXX-XX-XXXX"$"#)]
fn then_ssn_format(world: &mut FakerWorld) {
    let ssn = world.ssn.as_ref().expect("No SSN was generated");
    assert_eq!(ssn.len(), 11, "SSN '{}' is not 11 characters", ssn);
    assert_eq!(&ssn[3..4], "-", "SSN '{}' missing dash at position 3", ssn);
    assert_eq!(&ssn[6..7], "-", "SSN '{}' missing dash at position 6", ssn);
}

#[then(regex = r"^the SSN should contain exactly (\d+) digits$")]
fn then_ssn_digit_count(world: &mut FakerWorld, count: usize) {
    let ssn = world.ssn.as_ref().expect("No SSN was generated");
    let digit_count = ssn.chars().filter(|c| c.is_ascii_digit()).count();
    assert_eq!(
        digit_count, count,
        "SSN '{}' has {} digits but expected {}",
        ssn, digit_count, count
    );
}

// ============================================================================
// Then Steps — MRN Generation
// ============================================================================

#[then(regex = r"^the MRN should be between (\d+) and (\d+) digits long$")]
fn then_mrn_length_range(world: &mut FakerWorld, min: usize, max: usize) {
    let mrn = world.mrn.as_ref().expect("No MRN was generated");
    assert!(
        (min..=max).contains(&mrn.len()),
        "MRN '{}' has length {} but expected between {} and {}",
        mrn,
        mrn.len(),
        min,
        max
    );
}

#[then("the MRN should contain only digits")]
fn then_mrn_all_digits(world: &mut FakerWorld) {
    let mrn = world.mrn.as_ref().expect("No MRN was generated");
    assert!(
        mrn.chars().all(|c| c.is_ascii_digit()),
        "MRN '{}' contains non-digit characters",
        mrn
    );
}

// ============================================================================
// Then Steps — ICD-10 Generation
// ============================================================================

#[then(regex = r#"^the ICD-10 code should match the format "LNN\.N"$"#)]
fn then_icd10_format(world: &mut FakerWorld) {
    let code = world.icd10.as_ref().expect("No ICD-10 code was generated");
    assert!(code.contains('.'), "ICD-10 '{}' has no decimal point", code);
    let parts: Vec<&str> = code.split('.').collect();
    assert_eq!(parts.len(), 2, "ICD-10 '{}' format is wrong", code);
    assert_eq!(
        parts[0].len(),
        3,
        "ICD-10 category '{}' is not 3 chars",
        parts[0]
    );
    assert_eq!(
        parts[1].len(),
        1,
        "ICD-10 subcode '{}' is not 1 char",
        parts[1]
    );
}

#[then("the ICD-10 code should start with an uppercase letter")]
fn then_icd10_starts_with_letter(world: &mut FakerWorld) {
    let code = world.icd10.as_ref().expect("No ICD-10 code was generated");
    let first = code.chars().next().expect("ICD-10 code is empty");
    assert!(
        first.is_ascii_uppercase(),
        "ICD-10 '{}' does not start with an uppercase letter",
        code
    );
}

// ============================================================================
// Then Steps — LOINC Generation
// ============================================================================

#[then(regex = r"^the LOINC code should be between (\d+) and (\d+) digits long$")]
fn then_loinc_length_range(world: &mut FakerWorld, min: usize, max: usize) {
    let code = world.loinc.as_ref().expect("No LOINC code was generated");
    assert!(
        (min..=max).contains(&code.len()),
        "LOINC '{}' has length {} but expected between {} and {}",
        code,
        code.len(),
        min,
        max
    );
}

#[then("the LOINC code should contain only digits")]
fn then_loinc_all_digits(world: &mut FakerWorld) {
    let code = world.loinc.as_ref().expect("No LOINC code was generated");
    assert!(
        code.chars().all(|c| c.is_ascii_digit()),
        "LOINC '{}' contains non-digit characters",
        code
    );
}

// ============================================================================
// Then Steps — Medication and Allergen Generation
// ============================================================================

#[then("the medication should be a known medication")]
fn then_medication_known(world: &mut FakerWorld) {
    let med = world
        .medication
        .as_ref()
        .expect("No medication was generated");
    let medications = [
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
    assert!(
        medications.contains(&med.as_str()),
        "Medication '{}' is not in the known list",
        med
    );
}

#[then("the allergen should be a known allergen")]
fn then_allergen_known(world: &mut FakerWorld) {
    let allergen = world.allergen.as_ref().expect("No allergen was generated");
    let allergens = [
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
    assert!(
        allergens.contains(&allergen.as_str()),
        "Allergen '{}' is not in the known list",
        allergen
    );
}

// ============================================================================
// Then Steps — Patient Demographics
// ============================================================================

#[then("the blood type should be a valid ABO/Rh type")]
fn then_blood_type_valid(world: &mut FakerWorld) {
    let bt = world
        .blood_type
        .as_ref()
        .expect("No blood type was generated");
    let valid = ["A+", "A-", "B+", "B-", "AB+", "AB-", "O+", "O-"];
    assert!(
        valid.contains(&bt.as_str()),
        "Blood type '{}' is not valid",
        bt
    );
}

#[then("the ethnicity should be a recognized value")]
fn then_ethnicity_recognized(world: &mut FakerWorld) {
    let eth = world
        .ethnicity
        .as_ref()
        .expect("No ethnicity was generated");
    let valid = [
        "Hispanic or Latino",
        "Not Hispanic or Latino",
        "Declined to Specify",
    ];
    assert!(
        valid.contains(&eth.as_str()),
        "Ethnicity '{}' is not recognized",
        eth
    );
}

#[then("the race should be a recognized value")]
fn then_race_recognized(world: &mut FakerWorld) {
    let race = world.race.as_ref().expect("No race was generated");
    let valid = [
        "American Indian or Alaska Native",
        "Asian",
        "Black or African American",
        "Native Hawaiian or Other Pacific Islander",
        "White",
        "Declined to Specify",
    ];
    assert!(
        valid.contains(&race.as_str()),
        "Race '{}' is not recognized",
        race
    );
}

// ============================================================================
// Then Steps — Numeric Generation
// ============================================================================

#[then(regex = r"^the numeric string should be exactly (\d+) characters long$")]
fn then_numeric_length(world: &mut FakerWorld, length: usize) {
    let num = world
        .numeric_string
        .as_ref()
        .expect("No numeric string was generated");
    assert_eq!(
        num.len(),
        length,
        "Numeric string '{}' has length {} but expected {}",
        num,
        num.len(),
        length
    );
}

#[then("the numeric string should contain only digits")]
fn then_numeric_all_digits(world: &mut FakerWorld) {
    let num = world
        .numeric_string
        .as_ref()
        .expect("No numeric string was generated");
    assert!(
        num.chars().all(|c| c.is_ascii_digit()),
        "Numeric string '{}' contains non-digit characters",
        num
    );
}

// ============================================================================
// Then Steps — Date Generation
// ============================================================================

#[then("the date should be 8 digits in YYYYMMDD format")]
fn then_date_format(world: &mut FakerWorld) {
    let date = world
        .date_result
        .as_ref()
        .expect("No date result")
        .as_ref()
        .expect("Date generation failed unexpectedly");
    assert_eq!(date.len(), 8, "Date '{}' is not 8 characters", date);
    assert!(
        date.chars().all(|c| c.is_ascii_digit()),
        "Date '{}' is not all digits",
        date
    );
}

#[then(regex = r#"^the date should be between "([^"]*)" and "([^"]*)"$"#)]
fn then_date_in_range(world: &mut FakerWorld, start: String, end: String) {
    let date = world
        .date_result
        .as_ref()
        .expect("No date result")
        .as_ref()
        .expect("Date generation failed unexpectedly");
    assert!(
        date.as_str() >= start.as_str() && date.as_str() <= end.as_str(),
        "Date '{}' is not between '{}' and '{}'",
        date,
        start,
        end
    );
}

#[then(regex = r#"^the date should be "([^"]*)"$"#)]
fn then_date_equals(world: &mut FakerWorld, expected: String) {
    let date = world
        .date_result
        .as_ref()
        .expect("No date result")
        .as_ref()
        .expect("Date generation failed unexpectedly");
    assert_eq!(date, &expected, "Date '{}' != '{}'", date, expected);
}

#[then("the date generation should fail with an invalid format error")]
fn then_date_invalid_error(world: &mut FakerWorld) {
    let result = world.date_result.as_ref().expect("No date result");
    assert!(result.is_err(), "Expected date generation to fail");
    assert!(
        matches!(
            result.as_ref().unwrap_err(),
            hl7v2_faker::DateError::InvalidDateFormat(_)
        ),
        "Expected InvalidDateFormat error"
    );
}

// ============================================================================
// Then Steps — Gaussian Generation
// ============================================================================

#[then("the Gaussian value should be a valid number")]
fn then_gaussian_valid(world: &mut FakerWorld) {
    let value = world
        .gaussian_value
        .as_ref()
        .expect("No Gaussian value was generated");
    let _: f64 = value
        .parse()
        .unwrap_or_else(|_| panic!("Gaussian value '{}' is not a valid number", value));
}

#[then("the Gaussian value should be within 5 standard deviations of the mean")]
fn then_gaussian_within_range(world: &mut FakerWorld) {
    let value = world
        .gaussian_value
        .as_ref()
        .expect("No Gaussian value was generated");
    let parsed: f64 = value.parse().expect("Not a valid number");
    let low = world.gaussian_mean - 5.0 * world.gaussian_sd;
    let high = world.gaussian_mean + 5.0 * world.gaussian_sd;
    assert!(
        (low..=high).contains(&parsed),
        "Gaussian value {} is not within 5 SDs of mean {} (range {}-{})",
        parsed,
        world.gaussian_mean,
        low,
        high
    );
}

// ============================================================================
// Then Steps — UUID Generation
// ============================================================================

#[then(regex = r"^the UUID should be (\d+) characters long$")]
fn then_uuid_length(world: &mut FakerWorld, length: usize) {
    let uuid = world.uuid.as_ref().expect("No UUID was generated");
    assert_eq!(
        uuid.len(),
        length,
        "UUID '{}' has length {} but expected {}",
        uuid,
        uuid.len(),
        length
    );
}

#[then(regex = r"^the UUID should have dashes at positions (\d+), (\d+), (\d+), and (\d+)$")]
fn then_uuid_dash_positions(world: &mut FakerWorld, p1: usize, p2: usize, p3: usize, p4: usize) {
    let uuid = world.uuid.as_ref().expect("No UUID was generated");
    let chars: Vec<char> = uuid.chars().collect();
    assert_eq!(chars[p1], '-', "UUID '{}' no dash at position {}", uuid, p1);
    assert_eq!(chars[p2], '-', "UUID '{}' no dash at position {}", uuid, p2);
    assert_eq!(chars[p3], '-', "UUID '{}' no dash at position {}", uuid, p3);
    assert_eq!(chars[p4], '-', "UUID '{}' no dash at position {}", uuid, p4);
}

// ============================================================================
// Then Steps — Timestamp Generation
// ============================================================================

#[then("the timestamp should be 14 digits in YYYYMMDDHHMMSS format")]
fn then_timestamp_format(world: &mut FakerWorld) {
    let ts = world
        .timestamp
        .as_ref()
        .expect("No timestamp was generated");
    assert_eq!(ts.len(), 14, "Timestamp '{}' is not 14 characters", ts);
    assert!(
        ts.chars().all(|c| c.is_ascii_digit()),
        "Timestamp '{}' is not all digits",
        ts
    );
}

#[then(regex = r#"^the timestamp should start with "([^"]*)"$"#)]
fn then_timestamp_starts_with(world: &mut FakerWorld, prefix: String) {
    let ts = world
        .timestamp
        .as_ref()
        .expect("No timestamp was generated");
    assert!(
        ts.starts_with(&prefix),
        "Timestamp '{}' does not start with '{}'",
        ts,
        prefix
    );
}

// ============================================================================
// Then Steps — Selection Functions
// ============================================================================

#[then(regex = r#"^the selected value should be one of "([^"]*)"$"#)]
fn then_selected_value_one_of(world: &mut FakerWorld, options_str: String) {
    let options: Vec<&str> = options_str.split(',').collect();
    let value = world
        .selected_value
        .as_ref()
        .expect("No selection was made")
        .as_ref()
        .expect("Selection returned None");
    assert!(
        options.contains(&value.as_str()),
        "Selected value '{}' is not one of {:?}",
        value,
        options
    );
}

#[then("the selection should return nothing")]
fn then_selection_none(world: &mut FakerWorld) {
    let result = world
        .selected_value
        .as_ref()
        .expect("No selection was made");
    assert!(result.is_none(), "Expected None but got {:?}", result);
}

#[then(regex = r#"^the selected value should be one of the map values "([^"]*)"$"#)]
fn then_map_selected_value_one_of(world: &mut FakerWorld, values_str: String) {
    let values: Vec<&str> = values_str.split(',').collect();
    let value = world
        .map_selected_value
        .as_ref()
        .expect("No map selection was made")
        .as_ref()
        .expect("Map selection returned None");
    assert!(
        values.contains(&value.as_str()),
        "Selected map value '{}' is not one of {:?}",
        value,
        values
    );
}

#[then("the map selection should return nothing")]
fn then_map_selection_none(world: &mut FakerWorld) {
    let result = world
        .map_selected_value
        .as_ref()
        .expect("No map selection was made");
    assert!(result.is_none(), "Expected None but got {:?}", result);
}

// ============================================================================
// Then Steps — FakerValue Enum Generation
// ============================================================================

#[then(regex = r#"^the generated value should be "([^"]*)"$"#)]
fn then_generated_value_equals(world: &mut FakerWorld, expected: String) {
    let result = world
        .faker_value_result
        .as_ref()
        .expect("No FakerValue result");
    let value = result.as_ref().expect("FakerValue generation failed");
    assert_eq!(
        value, &expected,
        "Generated '{}' but expected '{}'",
        value, expected
    );
}

#[then(regex = r#"^the generated value should be one of "([^"]*)"$"#)]
fn then_generated_value_one_of(world: &mut FakerWorld, options_str: String) {
    let options: Vec<&str> = options_str.split(',').collect();
    let result = world
        .faker_value_result
        .as_ref()
        .expect("No FakerValue result");
    let value = result.as_ref().expect("FakerValue generation failed");
    assert!(
        options.contains(&value.as_str()),
        "Generated value '{}' is not one of {:?}",
        value,
        options
    );
}

#[then("the generation should fail with an empty options error")]
fn then_generation_fails_empty_options(world: &mut FakerWorld) {
    let result = world
        .faker_value_result
        .as_ref()
        .expect("No FakerValue result");
    assert!(result.is_err(), "Expected error but got success");
    assert!(
        matches!(result.as_ref().unwrap_err(), GenerateError::EmptyOptions),
        "Expected EmptyOptions error but got {:?}",
        result
    );
}

#[then(regex = r"^the generated value should be (\d+) characters long$")]
fn then_generated_value_length(world: &mut FakerWorld, length: usize) {
    let result = world
        .faker_value_result
        .as_ref()
        .expect("No FakerValue result");
    let value = result.as_ref().expect("FakerValue generation failed");
    assert_eq!(
        value.len(),
        length,
        "Generated value '{}' has length {} but expected {}",
        value,
        value.len(),
        length
    );
}

#[then("the generated value should contain only digits")]
fn then_generated_value_all_digits(world: &mut FakerWorld) {
    let result = world
        .faker_value_result
        .as_ref()
        .expect("No FakerValue result");
    let value = result.as_ref().expect("FakerValue generation failed");
    assert!(
        value.chars().all(|c| c.is_ascii_digit()),
        "Generated value '{}' contains non-digit characters",
        value
    );
}

#[then(regex = r#"^the generated value should contain "([^"]*)"$"#)]
fn then_generated_value_contains(world: &mut FakerWorld, substring: String) {
    let result = world
        .faker_value_result
        .as_ref()
        .expect("No FakerValue result");
    let value = result.as_ref().expect("FakerValue generation failed");
    assert!(
        value.contains(&substring),
        "Generated value '{}' does not contain '{}'",
        value,
        substring
    );
}

#[then(regex = r#"^the generated value should start with "([^"]*)"$"#)]
fn then_generated_value_starts_with(world: &mut FakerWorld, prefix: String) {
    let result = world
        .faker_value_result
        .as_ref()
        .expect("No FakerValue result");
    let value = result.as_ref().expect("FakerValue generation failed");
    assert!(
        value.starts_with(&prefix),
        "Generated value '{}' does not start with '{}'",
        value,
        prefix
    );
}

#[then(regex = r"^the generated value length should be between (\d+) and (\d+)$")]
fn then_generated_value_length_range(world: &mut FakerWorld, min: usize, max: usize) {
    let result = world
        .faker_value_result
        .as_ref()
        .expect("No FakerValue result");
    let value = result.as_ref().expect("FakerValue generation failed");
    assert!(
        (min..=max).contains(&value.len()),
        "Generated value '{}' has length {} but expected between {} and {}",
        value,
        value.len(),
        min,
        max
    );
}

#[then("the generation should fail with an empty map error")]
fn then_generation_fails_empty_map(world: &mut FakerWorld) {
    let result = world
        .faker_value_result
        .as_ref()
        .expect("No FakerValue result");
    assert!(result.is_err(), "Expected error but got success");
    assert!(
        matches!(result.as_ref().unwrap_err(), GenerateError::EmptyMap),
        "Expected EmptyMap error but got {:?}",
        result
    );
}

// ============================================================================
// Then Steps — Determinism
// ============================================================================

#[then("both sets of generated data should be identical")]
fn then_both_data_sets_identical(world: &mut FakerWorld) {
    let set1 = world
        .all_data_set1
        .as_ref()
        .expect("No first data set generated");
    let set2 = world
        .all_data_set2
        .as_ref()
        .expect("No second data set generated");
    assert_eq!(
        set1, set2,
        "Data sets differ:\nSet 1: {:?}\nSet 2: {:?}",
        set1, set2
    );
}

// ============================================================================
// Cucumber Main
// ============================================================================

#[tokio::main]
async fn main() {
    FakerWorld::run("tests/features/faker.feature").await;
}
