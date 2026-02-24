#![no_main]

use hl7v2_template_values::{generate_value, ValueSource};
use libfuzzer_sys::fuzz_target;
use rand::SeedableRng;
use rand::rngs::StdRng;

fuzz_target!(|data: &[u8]| {
    if let Ok(source) = serde_json::from_slice::<ValueSource>(data) {
        let mut rng = StdRng::seed_from_u64(0xDEADBEEF);
        let _ = generate_value(&source, &mut rng);
    }
});
