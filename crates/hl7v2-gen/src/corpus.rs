use hl7v2_core::{Error, Message};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::generate::generate_single_message;
use crate::template::Template;

/// Generate a corpus of messages
///
/// This function generates a large set of HL7 messages with varying characteristics
/// for testing and benchmarking purposes.
///
/// # Arguments
///
/// * `template` - The template to use for generating messages
/// * `seed` - The random seed for deterministic generation
/// * `count` - The number of messages to generate
/// * `batch_size` - The number of messages to generate in each batch
///
/// # Returns
///
/// A vector of generated messages
pub fn generate_corpus(template: &Template, seed: u64, count: usize, batch_size: usize) -> Result<Vec<Message>, Error> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut messages = Vec::with_capacity(count);

    // Generate messages in batches to manage memory usage
    let mut generated = 0;
    while generated < count {
        let batch_count = std::cmp::min(batch_size, count - generated);
        for _ in 0..batch_count {
            let message = generate_single_message(template, &mut rng, generated)?;
            messages.push(message);
            generated += 1;
        }
    }

    Ok(messages)
}

/// Generate a diverse corpus with different message types
///
/// This function generates a corpus with different types of HL7 messages
/// (ADT, ORU, etc.) to provide comprehensive testing data.
///
/// # Arguments
///
/// * `templates` - A vector of templates to use for generating messages
/// * `seed` - The random seed for deterministic generation
/// * `count` - The number of messages to generate
///
/// # Returns
///
/// A vector of generated messages with different types
pub fn generate_diverse_corpus(templates: &[Template], seed: u64, count: usize) -> Result<Vec<Message>, Error> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut messages = Vec::with_capacity(count);

    for i in 0..count {
        // Select a random template
        let template_index = rng.gen_range(0..templates.len());
        let template = &templates[template_index];

        let message = generate_single_message(template, &mut rng, i)?;
        messages.push(message);
    }

    Ok(messages)
}

/// Generate a corpus with specific distributions
///
/// This function generates a corpus with specific distributions of message characteristics
/// (e.g., specific percentages of different message types, error rates, etc.)
///
/// # Arguments
///
/// * `template_distributions` - A vector of (template, percentage) pairs
/// * `seed` - The random seed for deterministic generation
/// * `count` - The number of messages to generate
///
/// # Returns
///
/// A vector of generated messages following the specified distributions
pub fn generate_distributed_corpus(
    template_distributions: &[(Template, f64)],
    seed: u64,
    count: usize,
) -> Result<Vec<Message>, Error> {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut messages = Vec::with_capacity(count);

    // Normalize percentages to ensure they sum to 1.0
    let total_percentage: f64 = template_distributions.iter().map(|(_, p)| *p).sum();
    let normalized_distributions: Vec<(Template, f64)> = template_distributions
        .iter()
        .map(|(t, p)| (t.clone(), p / total_percentage))
        .collect();

    // Create cumulative distribution
    let mut cumulative_distribution = Vec::new();
    let mut cumulative = 0.0;
    for (template, percentage) in &normalized_distributions {
        cumulative += percentage;
        cumulative_distribution.push((template.clone(), cumulative));
    }

    for i in 0..count {
        // Select template based on distribution
        let random_value = rng.gen_range(0.0..1.0);
        let template = cumulative_distribution
            .iter()
            .find(|(_, cumulative)| random_value <= *cumulative)
            .map(|(t, _)| t)
            .unwrap_or(&normalized_distributions.last().unwrap().0);

        let message = generate_single_message(template, &mut rng, i)?;
        messages.push(message);
    }

    Ok(messages)
}
