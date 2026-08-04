//! Bounded reuse of compiled profile regexes.

use regex::Regex;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

const MAX_CACHED_PATTERNS: usize = 256;

#[derive(Debug)]
struct RegexCache {
    entries: Mutex<HashMap<String, Regex>>,
}

impl RegexCache {
    fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
        }
    }

    fn get_or_compile(&self, pattern: &str) -> Option<Regex> {
        let Ok(mut entries) = self.entries.lock() else {
            return Regex::new(pattern).ok();
        };

        if let Some(regex) = entries.get(pattern) {
            return Some(regex.clone());
        }

        let regex = Regex::new(pattern).ok()?;
        if entries.len() >= MAX_CACHED_PATTERNS {
            entries.clear();
        }
        entries.insert(pattern.to_owned(), regex.clone());
        Some(regex)
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.entries
            .lock()
            .map(|entries| entries.len())
            .unwrap_or(0)
    }
}

static PROFILE_REGEX_CACHE: OnceLock<RegexCache> = OnceLock::new();

/// Return a compiled profile regex, reusing successful compilations.
pub(crate) fn get(pattern: &str) -> Option<Regex> {
    PROFILE_REGEX_CACHE
        .get_or_init(RegexCache::new)
        .get_or_compile(pattern)
}

#[cfg(test)]
mod tests {
    use super::RegexCache;

    #[test]
    fn reuses_a_compiled_pattern_and_keeps_invalid_patterns_out() {
        let cache = RegexCache::new();

        assert!(cache.get_or_compile(r"^MRN\d+$").is_some());
        assert!(cache.get_or_compile(r"^MRN\d+$").is_some());
        assert!(cache.get_or_compile("[").is_none());
        assert_eq!(cache.len(), 1);
    }
}
