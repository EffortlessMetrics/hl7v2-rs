use hl7v2_core::Error;
use sha2::{Digest, Sha256};

use crate::generate::generate;
use crate::template::Template;

/// Verify that generated messages match expected hash values
///
/// This function generates messages and verifies that their SHA-256 hashes
/// match the expected golden hash values. This is useful for testing and
/// validation purposes.
///
/// # Arguments
///
/// * `template` - The template to use for generating messages
/// * `seed` - The random seed for deterministic generation
/// * `count` - The number of messages to generate
/// * `expected_hashes` - A vector of expected SHA-256 hash values
///
/// # Returns
///
/// A vector of booleans indicating whether each message's hash matches the expected hash
pub fn verify_golden_hashes(
    template: &Template,
    seed: u64,
    count: usize,
    expected_hashes: &[String],
) -> Result<Vec<bool>, Error> {
    // Generate messages
    let messages = generate(template, seed, count)?;

    // Verify each message against its expected hash
    let mut results = Vec::with_capacity(count);
    for (i, message) in messages.iter().enumerate() {
        if i < expected_hashes.len() {
            // Convert message to string
            let message_string = hl7v2_core::write(message);

            // Calculate SHA-256 hash
            let mut hasher = Sha256::new();
            hasher.update(&message_string);
            let hash_result = hasher.finalize();
            let hash_hex = format!("{:x}", hash_result);

            // Compare with expected hash
            results.push(hash_hex == expected_hashes[i]);
        } else {
            // No expected hash provided for this message
            results.push(false);
        }
    }

    Ok(results)
}

/// Generate golden hash values for a template
///
/// This function generates messages and returns their SHA-256 hash values
/// for use as golden hashes in future verification.
///
/// # Arguments
///
/// * `template` - The template to use for generating messages
/// * `seed` - The random seed for deterministic generation
/// * `count` - The number of messages to generate
///
/// # Returns
///
/// A vector of SHA-256 hash values for the generated messages
pub fn generate_golden_hashes(template: &Template, seed: u64, count: usize) -> Result<Vec<String>, Error> {
    // Generate messages
    let messages = generate(template, seed, count)?;

    // Calculate hash for each message
    let mut hashes = Vec::with_capacity(count);
    for message in messages.iter() {
        // Convert message to string
        let message_string = hl7v2_core::write(message);

        // Calculate SHA-256 hash
        let mut hasher = Sha256::new();
        hasher.update(&message_string);
        let hash_result = hasher.finalize();
        let hash_hex = format!("{:x}", hash_result);

        hashes.push(hash_hex);
    }

    Ok(hashes)
}
