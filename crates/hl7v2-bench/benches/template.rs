//! Benchmarks for HL7 v2 template-based message generation
//!
//! This benchmark suite profiles template performance:
//! - Template parsing from YAML
//! - Message generation from template
//! - Value source resolution

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use hl7v2_template::{Template, ValueSource, generate, generate_corpus};
use hl7v2_template_values::generate_value;
use rand::RngExt;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::collections::HashMap;
use std::hint::black_box;

/// Create a simple ADT_A01 template
fn create_simple_template() -> Template {
    Template {
        name: "ADT_A01_Simple".to_string(),
        delims: "^~\\&".to_string(),
        segments: vec![
            "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|MSG00001|P|2.5.1".to_string(),
            "PID|1||123456^^^HOSP^MR||Doe^John^A||19800101|M".to_string(),
        ],
        values: HashMap::new(),
    }
}

/// Create a template with value sources
fn create_template_with_values() -> Template {
    let mut values = HashMap::new();

    // MSH.7 - Message timestamp
    values.insert("MSH.7".to_string(), vec![ValueSource::DtmNowUtc]);

    // MSH.10 - Message control ID
    values.insert("MSH.10".to_string(), vec![ValueSource::UuidV4]);

    // PID.3 - Patient MRN
    values.insert("PID.3".to_string(), vec![ValueSource::RealisticMrn]);

    // PID.5 - Patient name
    values.insert(
        "PID.5".to_string(),
        vec![ValueSource::RealisticName { gender: None }],
    );

    // PID.7 - Date of birth
    values.insert(
        "PID.7".to_string(),
        vec![ValueSource::Date {
            start: "19500101".to_string(),
            end: "20101231".to_string(),
        }],
    );

    // PID.11 - Address
    values.insert("PID.11".to_string(), vec![ValueSource::RealisticAddress]);

    // PID.13 - Phone
    values.insert("PID.13".to_string(), vec![ValueSource::RealisticPhone]);

    // PID.19 - SSN
    values.insert("PID.19".to_string(), vec![ValueSource::RealisticSsn]);

    Template {
        name: "ADT_A01_WithValues".to_string(),
        delims: "^~\\&".to_string(),
        segments: vec![
            "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|MSG00001|P|2.5.1".to_string(),
            "PID|1||123456^^^HOSP^MR||Doe^John^A||19800101|M||C|123 Main St^^Anytown^ST^12345||(555)555-1212||E||S||123456789|".to_string(),
        ],
        values,
    }
}

/// Create a complex template with many segments
fn create_complex_template() -> Template {
    let mut values = HashMap::new();

    values.insert("MSH.7".to_string(), vec![ValueSource::DtmNowUtc]);
    values.insert("MSH.10".to_string(), vec![ValueSource::UuidV4]);
    values.insert("PID.3".to_string(), vec![ValueSource::RealisticMrn]);
    values.insert(
        "PID.5".to_string(),
        vec![ValueSource::RealisticName { gender: None }],
    );
    values.insert(
        "PID.7".to_string(),
        vec![ValueSource::Date {
            start: "19500101".to_string(),
            end: "20101231".to_string(),
        }],
    );
    values.insert("PID.11".to_string(), vec![ValueSource::RealisticAddress]);
    values.insert("PID.13".to_string(), vec![ValueSource::RealisticPhone]);
    values.insert("PID.19".to_string(), vec![ValueSource::RealisticSsn]);

    // OBX values
    values.insert(
        "OBX.5".to_string(),
        vec![ValueSource::Gaussian {
            mean: 100.0,
            sd: 15.0,
            precision: 2,
        }],
    );

    // AL1 values
    values.insert("AL1.3".to_string(), vec![ValueSource::RealisticAllergen]);

    // DG1 values
    values.insert("DG1.3".to_string(), vec![ValueSource::RealisticIcd10]);

    Template {
        name: "ADT_A01_Complex".to_string(),
        delims: "^~\\&".to_string(),
        segments: vec![
            "MSH|^~\\&|SendingApp|SendingFac|ReceivingApp|ReceivingFac|20250128152312||ADT^A01^ADT_A01|MSG00001|P|2.5.1".to_string(),
            "PID|1||123456^^^HOSP^MR||Doe^John^A||19800101|M||C|123 Main St^^Anytown^ST^12345||(555)555-1212||E||S||123456789|".to_string(),
            "PV1|1|I|ICU^101^01||||DOE^JOHN^A^III^^^^MD|||SUR||||||ADM|12345678|||||||||||||||||||||||||20250128120000".to_string(),
            "OBX|1|NM|HEIGHT^Height^L||180|cm|||||F".to_string(),
            "OBX|2|NM|WEIGHT^Weight^L||75|kg|||||F".to_string(),
            "OBX|3|NM|HR^Heart Rate^L||72|bpm|||||F".to_string(),
            "OBX|4|NM|TEMP^Temperature^L||37.0|C|||||F".to_string(),
            "OBX|5|NM|SPO2^Oxygen Saturation^L||98|%|||||F".to_string(),
            "AL1|1|DA|PENICILLIN^Penicillin^L||RASH||20200101".to_string(),
            "AL1|2|DA|ASPIRIN^Aspirin^L||ANAPHYLAXIS||20190101".to_string(),
            "DG1|1|ICD10|J18.9^Pneumonia||20250128||A".to_string(),
            "DG1|2|ICD10|E11.9^Type 2 Diabetes||20240101||A".to_string(),
        ],
        values,
    }
}

/// Create a template with numeric value sources
fn create_numeric_template() -> Template {
    let mut values = HashMap::new();

    values.insert(
        "OBX.5".to_string(),
        vec![ValueSource::Gaussian {
            mean: 100.0,
            sd: 15.0,
            precision: 2,
        }],
    );
    values.insert(
        "OBX.6".to_string(),
        vec![ValueSource::From(vec![
            "mg/dL".to_string(),
            "mmol/L".to_string(),
            "U/L".to_string(),
        ])],
    );

    Template {
        name: "NumericTemplate".to_string(),
        delims: "^~\\&".to_string(),
        segments: vec![
            "MSH|^~\\&|App|Fac|App2|Fac2|20250128120000||ORU^R01|MSG001|P|2.5.1".to_string(),
            "PID|1||12345^^^HOSP^MR||Test^Patient||19900101|M".to_string(),
            "OBX|1|NM|TEST^Test^L||100|units|||||F".to_string(),
        ],
        values,
    }
}

/// Benchmark simple template message generation
fn bench_simple_template_generation(c: &mut Criterion) {
    let template = create_simple_template();

    c.bench_function("simple_template_generation", |b| {
        b.iter(|| {
            let result = generate(black_box(&template), 42, 1);
            black_box(result)
        })
    });
}

/// Benchmark template with value sources
fn bench_template_with_values_generation(c: &mut Criterion) {
    let template = create_template_with_values();

    c.bench_function("template_with_values_generation", |b| {
        b.iter(|| {
            let result = generate(black_box(&template), 42, 1);
            black_box(result)
        })
    });
}

/// Benchmark complex template generation
fn bench_complex_template_generation(c: &mut Criterion) {
    let template = create_complex_template();

    c.bench_function("complex_template_generation", |b| {
        b.iter(|| {
            let result = generate(black_box(&template), 42, 1);
            black_box(result)
        })
    });
}

/// Benchmark message generation throughput
fn bench_generation_throughput(c: &mut Criterion) {
    let template = create_template_with_values();

    let mut group = c.benchmark_group("generation_throughput");

    for count in [1, 10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*count as u64));
        group.bench_with_input(BenchmarkId::new("messages", count), count, |b, &count| {
            b.iter(|| {
                let result = generate(black_box(&template), 42, count);
                black_box(result)
            })
        });
    }

    group.finish();
}

/// Benchmark value source resolution
fn bench_value_source_resolution(c: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(42);

    let mut group = c.benchmark_group("value_source_resolution");

    // Fixed value
    let fixed = ValueSource::Fixed("TestValue".to_string());
    group.bench_function("fixed", |b| {
        b.iter(|| {
            let result = generate_value(black_box(&fixed), &mut rng);
            black_box(result)
        })
    });

    // From list
    let from_list = ValueSource::From(vec![
        "Option1".to_string(),
        "Option2".to_string(),
        "Option3".to_string(),
    ]);
    group.bench_function("from_list", |b| {
        b.iter(|| {
            let result = generate_value(black_box(&from_list), &mut rng);
            black_box(result)
        })
    });

    // Numeric
    let numeric = ValueSource::Numeric { digits: 10 };
    group.bench_function("numeric", |b| {
        b.iter(|| {
            let result = generate_value(black_box(&numeric), &mut rng);
            black_box(result)
        })
    });

    // UUID v4
    let uuid = ValueSource::UuidV4;
    group.bench_function("uuid_v4", |b| {
        b.iter(|| {
            let result = generate_value(black_box(&uuid), &mut rng);
            black_box(result)
        })
    });

    // Date
    let date = ValueSource::Date {
        start: "20200101".to_string(),
        end: "20251231".to_string(),
    };
    group.bench_function("date", |b| {
        b.iter(|| {
            let result = generate_value(black_box(&date), &mut rng);
            black_box(result)
        })
    });

    // Gaussian
    let gaussian = ValueSource::Gaussian {
        mean: 100.0,
        sd: 15.0,
        precision: 2,
    };
    group.bench_function("gaussian", |b| {
        b.iter(|| {
            let result = generate_value(black_box(&gaussian), &mut rng);
            black_box(result)
        })
    });

    group.finish();
}

/// Benchmark realistic value generators
fn bench_realistic_value_generators(c: &mut Criterion) {
    let mut rng = StdRng::seed_from_u64(42);

    let mut group = c.benchmark_group("realistic_generators");

    // Realistic name
    let name = ValueSource::RealisticName { gender: None };
    group.bench_function("realistic_name", |b| {
        b.iter(|| {
            let result = generate_value(black_box(&name), &mut rng);
            black_box(result)
        })
    });

    // Realistic address
    let address = ValueSource::RealisticAddress;
    group.bench_function("realistic_address", |b| {
        b.iter(|| {
            let result = generate_value(black_box(&address), &mut rng);
            black_box(result)
        })
    });

    // Realistic phone
    let phone = ValueSource::RealisticPhone;
    group.bench_function("realistic_phone", |b| {
        b.iter(|| {
            let result = generate_value(black_box(&phone), &mut rng);
            black_box(result)
        })
    });

    // Realistic SSN
    let ssn = ValueSource::RealisticSsn;
    group.bench_function("realistic_ssn", |b| {
        b.iter(|| {
            let result = generate_value(black_box(&ssn), &mut rng);
            black_box(result)
        })
    });

    // Realistic MRN
    let mrn = ValueSource::RealisticMrn;
    group.bench_function("realistic_mrn", |b| {
        b.iter(|| {
            let result = generate_value(black_box(&mrn), &mut rng);
            black_box(result)
        })
    });

    // Realistic ICD-10
    let icd10 = ValueSource::RealisticIcd10;
    group.bench_function("realistic_icd10", |b| {
        b.iter(|| {
            let result = generate_value(black_box(&icd10), &mut rng);
            black_box(result)
        })
    });

    // Realistic LOINC
    let loinc = ValueSource::RealisticLoinc;
    group.bench_function("realistic_loinc", |b| {
        b.iter(|| {
            let result = generate_value(black_box(&loinc), &mut rng);
            black_box(result)
        })
    });

    // Realistic medication
    let med = ValueSource::RealisticMedication;
    group.bench_function("realistic_medication", |b| {
        b.iter(|| {
            let result = generate_value(black_box(&med), &mut rng);
            black_box(result)
        })
    });

    // Realistic allergen
    let allergen = ValueSource::RealisticAllergen;
    group.bench_function("realistic_allergen", |b| {
        b.iter(|| {
            let result = generate_value(black_box(&allergen), &mut rng);
            black_box(result)
        })
    });

    group.finish();
}

/// Benchmark corpus generation
fn bench_corpus_generation(c: &mut Criterion) {
    let template = create_template_with_values();

    let mut group = c.benchmark_group("corpus_generation");

    for count in [10, 100, 1000].iter() {
        group.throughput(Throughput::Elements(*count as u64));
        group.bench_with_input(BenchmarkId::new("corpus", count), count, |b, &count| {
            b.iter(|| {
                let result = generate_corpus(black_box(&template), 42, count, 100);
                black_box(result)
            })
        });
    }

    group.finish();
}

/// Benchmark template complexity impact
fn bench_template_complexity(c: &mut Criterion) {
    let mut group = c.benchmark_group("template_complexity");

    // Simple template (2 segments, no values)
    let simple = create_simple_template();
    group.bench_function("simple_2_segments", |b| {
        b.iter(|| {
            let result = generate(black_box(&simple), 42, 1);
            black_box(result)
        })
    });

    // Medium template (2 segments, 8 value sources)
    let medium = create_template_with_values();
    group.bench_function("medium_2_segments_8_values", |b| {
        b.iter(|| {
            let result = generate(black_box(&medium), 42, 1);
            black_box(result)
        })
    });

    // Complex template (12 segments, 15+ value sources)
    let complex = create_complex_template();
    group.bench_function("complex_12_segments_15_values", |b| {
        b.iter(|| {
            let result = generate(black_box(&complex), 42, 1);
            black_box(result)
        })
    });

    group.finish();
}

/// Benchmark deterministic generation (same seed = same output)
fn bench_deterministic_generation(c: &mut Criterion) {
    let template = create_template_with_values();

    c.bench_function("deterministic_generation", |b| {
        let mut iteration = 0u64;
        b.iter(|| {
            let result = generate(black_box(&template), iteration, 1);
            iteration += 1;
            black_box(result)
        })
    });
}

/// Benchmark batch generation with different batch sizes
fn bench_batch_generation(c: &mut Criterion) {
    let template = create_template_with_values();

    let mut group = c.benchmark_group("batch_generation");

    // Small batch
    group.bench_function("batch_10", |b| {
        b.iter(|| {
            let result = generate(black_box(&template), 42, 10);
            black_box(result)
        })
    });

    // Medium batch
    group.bench_function("batch_100", |b| {
        b.iter(|| {
            let result = generate(black_box(&template), 42, 100);
            black_box(result)
        })
    });

    // Large batch
    group.bench_function("batch_1000", |b| {
        b.iter(|| {
            let result = generate(black_box(&template), 42, 1000);
            black_box(result)
        })
    });

    group.finish();
}

/// Benchmark template with numeric value sources
fn bench_numeric_template(c: &mut Criterion) {
    let template = create_numeric_template();

    c.bench_function("numeric_template_generation", |b| {
        b.iter(|| {
            let result = generate(black_box(&template), 42, 1);
            black_box(result)
        })
    });
}

/// Benchmark value generation overhead
fn bench_value_generation_overhead(c: &mut Criterion) {
    let template_no_values = create_simple_template();
    let template_with_values = create_template_with_values();

    let mut group = c.benchmark_group("value_overhead");

    group.bench_function("no_values", |b| {
        b.iter(|| {
            let result = generate(black_box(&template_no_values), 42, 1);
            black_box(result)
        })
    });

    group.bench_function("with_values", |b| {
        b.iter(|| {
            let result = generate(black_box(&template_with_values), 42, 1);
            black_box(result)
        })
    });

    group.finish();
}

criterion_group!(
    template_benches,
    bench_simple_template_generation,
    bench_template_with_values_generation,
    bench_complex_template_generation,
    bench_generation_throughput,
    bench_value_source_resolution,
    bench_realistic_value_generators,
    bench_corpus_generation,
    bench_template_complexity,
    bench_deterministic_generation,
    bench_batch_generation,
    bench_numeric_template,
    bench_value_generation_overhead,
);

criterion_main!(template_benches);
