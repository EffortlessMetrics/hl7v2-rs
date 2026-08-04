// SPDX-License-Identifier: AGPL-3.0-or-later

//! Bounded reuse of compiled profile regexes.

use regex::Regex;
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

const MAX_CACHED_PATTERNS: usize = 256;

#[derive(Debug)]
struct RegexCache {
    entries: RwLock<HashMap<String, Regex>>,
}

impl RegexCache {
    fn new() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
        }
    }

    fn get_or_compile(&self, pattern: &str) -> Option<Regex> {
        if let Ok(entries) = self.entries.read()
            && let Some(regex) = entries.get(pattern)
        {
            return Some(regex.clone());
        }

        // Compilation is deliberately outside the write lock so an expensive
        // first-use pattern does not block cache hits for other patterns.
        let regex = Regex::new(pattern).ok()?;
        let Ok(mut entries) = self.entries.write() else {
            return Some(regex);
        };

        // Another thread may have compiled the same pattern while this one
        // was outside the lock. Prefer the cached value in that case.
        if let Some(cached) = entries.get(pattern) {
            return Some(cached.clone());
        }

        if entries.len() >= MAX_CACHED_PATTERNS
            && let Some(evicted_pattern) = entries.keys().next().cloned()
        {
            entries.remove(&evicted_pattern);
        }
        entries.insert(pattern.to_owned(), regex.clone());
        Some(regex)
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.entries
            .read()
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

    #[test]
    fn evicts_one_pattern_without_exceeding_the_bound() {
        let cache = RegexCache::new();

        for index in 0..=super::MAX_CACHED_PATTERNS {
            assert!(cache.get_or_compile(&format!(r"^value-{index}$")).is_some());
        }

        assert_eq!(cache.len(), super::MAX_CACHED_PATTERNS);
    }
}
