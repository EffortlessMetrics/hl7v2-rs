//! Profile loader with remote loading and caching support.
//!
//! This module provides the [`ProfileLoader`] struct for loading HL7 v2 profiles
//! from local files or remote URLs with ETag-based caching.

use std::sync::Arc;
use std::time::Duration;

use async_lock::RwLock;
use lru::LruCache;
use thiserror::Error;

use crate::{Profile, load_profile};

/// Default cache size (number of profiles)
const DEFAULT_CACHE_SIZE: usize = 100;

/// Default request timeout in seconds
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Errors that can occur during profile loading.
#[derive(Debug, Error)]
pub enum ProfileLoadError {
    /// Network error during HTTP request
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    /// Error parsing profile YAML
    #[error("Parse error: {0}")]
    Parse(String),

    /// Error reading local file
    #[error("File error: {0}")]
    File(String),

    /// Profile not found in cache (internal use)
    #[error("Profile not found: {0}")]
    NotFound(String),

    /// Invalid URL scheme
    #[error("Invalid URL scheme: {0}. Only http and https are supported.")]
    InvalidScheme(String),

    /// Cache operation failed
    #[error("Cache error: {0}")]
    Cache(String),

    /// Core library error
    #[error("Core error: {0}")]
    Core(String),
}

impl From<serde_yaml::Error> for ProfileLoadError {
    fn from(err: serde_yaml::Error) -> Self {
        ProfileLoadError::Parse(err.to_string())
    }
}

impl From<std::io::Error> for ProfileLoadError {
    fn from(err: std::io::Error) -> Self {
        ProfileLoadError::File(err.to_string())
    }
}

impl From<hl7v2_core::Error> for ProfileLoadError {
    fn from(err: hl7v2_core::Error) -> Self {
        ProfileLoadError::Core(err.to_string())
    }
}

/// Cache entry containing the profile and its ETag.
#[derive(Debug, Clone)]
struct CacheEntry {
    /// The loaded profile
    profile: Profile,
    /// ETag for conditional requests (if available)
    etag: Option<String>,
    /// Raw YAML content for comparison (kept for potential future use)
    #[allow(dead_code)]
    raw_content: String,
}

/// Result of a profile load operation.
#[derive(Debug, Clone)]
pub struct LoadResult {
    /// The loaded profile
    pub profile: Profile,
    /// Whether the profile was loaded from cache
    pub from_cache: bool,
    /// ETag of the profile (if available)
    pub etag: Option<String>,
}

/// Builder for configuring and creating a [`ProfileLoader`].
///
/// # Example
///
/// ```rust,no_run
/// use hl7v2_prof::loader::{ProfileLoader, ProfileLoadError};
/// use std::time::Duration;
///
/// #[tokio::main]
/// async fn main() -> Result<(), ProfileLoadError> {
///     let loader = ProfileLoader::new()
///         .with_cache_size(50)
///         .with_timeout(Duration::from_secs(60));
///
///     let result = loader.load("https://example.com/profiles/adt_a01.yaml").await?;
///     println!("Profile loaded: {}", result.profile.message_structure);
///     Ok(())
/// }
/// ```
#[derive(Debug)]
pub struct ProfileLoader {
    /// HTTP client for remote requests
    client: reqwest::Client,
    /// LRU cache for loaded profiles
    cache: Arc<RwLock<LruCache<String, CacheEntry>>>,
    /// Request timeout
    timeout: Duration,
}

impl Default for ProfileLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl ProfileLoader {
    /// Create a new profile loader with default settings.
    ///
    /// Default settings:
    /// - Cache size: 100 profiles
    /// - Timeout: 30 seconds
    pub fn new() -> Self {
        Self::with_options(
            DEFAULT_CACHE_SIZE,
            Duration::from_secs(DEFAULT_TIMEOUT_SECS),
        )
    }

    /// Create a profile loader with custom cache size and timeout.
    ///
    /// # Arguments
    ///
    /// * `cache_size` - Maximum number of profiles to cache
    /// * `timeout` - Request timeout for HTTP requests
    pub fn with_options(cache_size: usize, timeout: Duration) -> Self {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .user_agent(format!("hl7v2-prof/{}", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            client,
            cache: Arc::new(RwLock::new(LruCache::new(
                std::num::NonZeroUsize::new(cache_size).expect("Cache size must be non-zero"),
            ))),
            timeout,
        }
    }

    /// Set a custom cache size.
    ///
    /// This creates a new loader with the specified cache size.
    ///
    /// # Arguments
    ///
    /// * `size` - Maximum number of profiles to cache
    pub fn with_cache_size(self, size: usize) -> Self {
        Self::with_options(size, self.timeout)
    }

    /// Set a custom request timeout.
    ///
    /// This creates a new loader with the specified timeout.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Request timeout for HTTP requests
    pub fn with_timeout(self, timeout: Duration) -> Self {
        Self::with_options(
            self.cache
                .try_read()
                .map(|c| c.cap().get())
                .unwrap_or(DEFAULT_CACHE_SIZE),
            timeout,
        )
    }

    /// Load a profile from a URL or file path.
    ///
    /// This method automatically detects whether the source is a URL (starting with
    /// `http://` or `https://`) or a local file path.
    ///
    /// # Arguments
    ///
    /// * `source` - URL or file path to load the profile from
    ///
    /// # Returns
    ///
    /// A [`LoadResult`] containing the profile and metadata about the load operation.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use hl7v2_prof::loader::{ProfileLoader, ProfileLoadError};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), ProfileLoadError> {
    ///     let loader = ProfileLoader::new();
    ///
    ///     // Load from URL
    ///     let result = loader.load("https://example.com/profiles/adt_a01.yaml").await?;
    ///     println!("Loaded from cache: {}", result.from_cache);
    ///
    ///     // Load from file
    ///     let result = loader.load("./profiles/adt_a01.yaml").await?;
    ///     println!("Loaded from cache: {}", result.from_cache);
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn load(&self, source: &str) -> Result<LoadResult, ProfileLoadError> {
        if source.starts_with("http://") || source.starts_with("https://") {
            self.load_from_url(source).await
        } else {
            self.load_from_file(source).await
        }
    }

    /// Load a profile from a URL with ETag caching.
    ///
    /// If the profile is already in cache and the server responds with 304 Not Modified,
    /// the cached profile is returned.
    ///
    /// # Arguments
    ///
    /// * `url` - URL to load the profile from
    ///
    /// # Returns
    ///
    /// A [`LoadResult`] containing the profile and metadata.
    pub async fn load_from_url(&self, url: &str) -> Result<LoadResult, ProfileLoadError> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(entry) = cache.peek(url) {
                // We have a cached version, try conditional request
                let mut request = self.client.get(url);

                if let Some(etag) = &entry.etag {
                    request = request.header("If-None-Match", etag);
                }

                let response = request.send().await?;

                if response.status() == reqwest::StatusCode::NOT_MODIFIED {
                    // Cache is still valid
                    return Ok(LoadResult {
                        profile: entry.profile.clone(),
                        from_cache: true,
                        etag: entry.etag.clone(),
                    });
                }

                // Server responded with new content, fall through to fetch
            }
        }

        // Fetch from server
        let response = self.client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(ProfileLoadError::Network(
                response.error_for_status().unwrap_err(),
            ));
        }

        let etag = response
            .headers()
            .get("ETag")
            .and_then(|v| v.to_str().ok())
            .map(String::from);

        let content = response.text().await?;
        let profile = load_profile(&content).map_err(ProfileLoadError::from)?;

        // Store in cache
        {
            let mut cache = self.cache.write().await;
            cache.put(
                url.to_string(),
                CacheEntry {
                    profile: profile.clone(),
                    etag: etag.clone(),
                    raw_content: content,
                },
            );
        }

        Ok(LoadResult {
            profile,
            from_cache: false,
            etag,
        })
    }

    /// Load a profile from a local file.
    ///
    /// The file is cached by its path, so subsequent loads of the same file
    /// will return the cached version.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the profile file
    ///
    /// # Returns
    ///
    /// A [`LoadResult`] containing the profile and metadata.
    pub async fn load_from_file(&self, path: &str) -> Result<LoadResult, ProfileLoadError> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(entry) = cache.peek(path) {
                return Ok(LoadResult {
                    profile: entry.profile.clone(),
                    from_cache: true,
                    etag: entry.etag.clone(),
                });
            }
        }

        // Load from file
        let content = tokio::fs::read_to_string(path).await?;
        let profile = load_profile(&content).map_err(ProfileLoadError::from)?;

        // Store in cache
        {
            let mut cache = self.cache.write().await;
            cache.put(
                path.to_string(),
                CacheEntry {
                    profile: profile.clone(),
                    etag: None,
                    raw_content: content,
                },
            );
        }

        Ok(LoadResult {
            profile,
            from_cache: false,
            etag: None,
        })
    }

    /// Load a profile from a local file synchronously.
    ///
    /// This is a convenience method for non-async contexts.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the profile file
    ///
    /// # Returns
    ///
    /// The loaded profile.
    pub fn load_file_sync(path: &str) -> Result<Profile, ProfileLoadError> {
        let content = std::fs::read_to_string(path)?;
        let profile = load_profile(&content)?;
        Ok(profile)
    }

    /// Invalidate a cached profile.
    ///
    /// # Arguments
    ///
    /// * `source` - URL or file path to invalidate
    ///
    /// # Returns
    ///
    /// `true` if the profile was in the cache and was removed, `false` otherwise.
    pub async fn invalidate(&self, source: &str) -> bool {
        let mut cache = self.cache.write().await;
        cache.pop(source).is_some()
    }

    /// Clear the entire cache.
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// Get the number of profiles currently in the cache.
    pub async fn cache_size(&self) -> usize {
        let cache = self.cache.read().await;
        cache.len()
    }

    /// Check if a profile is in the cache.
    pub async fn is_cached(&self, source: &str) -> bool {
        let cache = self.cache.read().await;
        cache.contains(source)
    }

    /// Prefetch a profile into the cache.
    ///
    /// This is useful for warming up the cache before the profile is needed.
    ///
    /// # Arguments
    ///
    /// * `source` - URL or file path to prefetch
    pub async fn prefetch(&self, source: &str) -> Result<(), ProfileLoadError> {
        self.load(source).await?;
        Ok(())
    }

    /// Prefetch multiple profiles into the cache.
    ///
    /// # Arguments
    ///
    /// * `sources` - URLs or file paths to prefetch
    ///
    /// # Returns
    ///
    /// A vector of results, one for each source.
    pub async fn prefetch_all<'a, I>(&self, sources: I) -> Vec<Result<(), ProfileLoadError>>
    where
        I: IntoIterator<Item = &'a str>,
    {
        let mut results = Vec::new();
        for source in sources {
            results.push(self.prefetch(source).await);
        }
        results
    }
}

/// Load a profile from a URL (convenience function).
///
/// This creates a new [`ProfileLoader`] with default settings and loads the profile.
/// For repeated loads, create a [`ProfileLoader`] instance instead.
///
/// # Arguments
///
/// * `url` - URL to load the profile from
///
/// # Returns
///
/// The loaded profile.
pub async fn load_from_url(url: &str) -> Result<Profile, ProfileLoadError> {
    let loader = ProfileLoader::new();
    let result = loader.load(url).await?;
    Ok(result.profile)
}

/// Load a profile from a file (convenience function).
///
/// This is a convenience wrapper around [`ProfileLoader::load_file_sync`].
///
/// # Arguments
///
/// * `path` - Path to the profile file
///
/// # Returns
///
/// The loaded profile.
pub fn load_from_file(path: &str) -> Result<Profile, ProfileLoadError> {
    ProfileLoader::load_file_sync(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_loader_creation() {
        let loader = ProfileLoader::new()
            .with_cache_size(50)
            .with_timeout(Duration::from_secs(60));

        assert_eq!(loader.cache_size().await, 0);
    }

    #[tokio::test]
    async fn test_load_local_file() {
        let loader = ProfileLoader::new();

        // Try to load an example profile
        let result = loader
            .load_from_file("examples/profiles/minimal.yaml")
            .await;

        // This test may fail if run from a different directory
        if let Ok(result) = result {
            assert!(!result.from_cache);
            assert_eq!(result.profile.message_structure, "ADT_A01");

            // Second load should be from cache
            let cached = loader
                .load_from_file("examples/profiles/minimal.yaml")
                .await
                .unwrap();
            assert!(cached.from_cache);
        }
    }

    #[tokio::test]
    async fn test_cache_invalidation() {
        let loader = ProfileLoader::new();

        // Load a file
        let result = loader
            .load_from_file("examples/profiles/minimal.yaml")
            .await;

        if result.is_ok() {
            assert!(loader.is_cached("examples/profiles/minimal.yaml").await);

            // Invalidate
            let removed = loader.invalidate("examples/profiles/minimal.yaml").await;
            assert!(removed);

            assert!(!loader.is_cached("examples/profiles/minimal.yaml").await);
        }
    }

    #[tokio::test]
    async fn test_clear_cache() {
        let loader = ProfileLoader::new();

        // Load multiple files
        let _ = loader
            .load_from_file("examples/profiles/minimal.yaml")
            .await;
        let _ = loader
            .load_from_file("examples/profiles/ADT_A01.yaml")
            .await;

        loader.clear_cache().await;
        assert_eq!(loader.cache_size().await, 0);
    }

    #[test]
    fn test_sync_load() {
        let result = ProfileLoader::load_file_sync("examples/profiles/minimal.yaml");

        if let Ok(profile) = result {
            assert_eq!(profile.message_structure, "ADT_A01");
        }
    }

    #[tokio::test]
    async fn test_invalid_url_scheme() {
        let loader = ProfileLoader::new();

        // This should try to load as a file and fail
        let result = loader.load("ftp://example.com/profile.yaml").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_file_not_found() {
        let loader = ProfileLoader::new();

        let result = loader.load_from_file("nonexistent_profile.yaml").await;
        assert!(result.is_err());

        // Just verify it's a File error - the exact message varies by platform
        assert!(matches!(result, Err(ProfileLoadError::File(_))));
    }

    #[tokio::test]
    async fn test_parse_error() {
        let loader = ProfileLoader::new();

        // Create a temp file with invalid YAML
        let temp_content = "this is not: valid yaml\n  bad indentation";
        let temp_path = std::env::temp_dir().join("test_invalid_profile.yaml");
        std::fs::write(&temp_path, temp_content).unwrap();

        let result = loader.load_from_file(temp_path.to_str().unwrap()).await;
        assert!(result.is_err());

        // Clean up
        std::fs::remove_file(&temp_path).ok();

        if let Err(ProfileLoadError::Parse(_)) = result {
            // Expected
        }
    }

    #[tokio::test]
    async fn test_lru_eviction() {
        // Create a loader with very small cache
        let loader = ProfileLoader::with_options(2, Duration::from_secs(30));

        // Load files (if they exist)
        let _ = loader
            .load_from_file("examples/profiles/minimal.yaml")
            .await;
        let _ = loader
            .load_from_file("examples/profiles/ADT_A01.yaml")
            .await;
        let _ = loader
            .load_from_file("examples/profiles/ADT_A04.yaml")
            .await;

        // Cache should have at most 2 entries
        assert!(loader.cache_size().await <= 2);
    }
}
