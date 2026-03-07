//! Profile loader with remote loading and caching support.
//!
//! This module provides the [`ProfileLoader`](crate::loader::ProfileLoader) struct for loading HL7 v2 profiles
//! from local files or remote URLs with ETag-based caching.

use std::sync::Arc;
use std::time::Duration;

use async_lock::RwLock;
use lru::LruCache;

pub use crate::ProfileLoadError;
use crate::{Profile, load_profile};

/// Default cache size (number of profiles)
const DEFAULT_CACHE_SIZE: usize = 100;

/// Default request timeout in seconds
const DEFAULT_TIMEOUT_SECS: u64 = 30;

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
/// use hl7v2_prof::loader::ProfileLoader;
/// use hl7v2_prof::ProfileLoadError;
/// use std::time::Duration;
///
/// #[tokio::main]
/// async fn main() -> Result<(), ProfileLoadError> {
///     let loader = ProfileLoader::builder()
///         .cache_size(50)
///         .timeout(Duration::from_secs(10))
///         .build();
///     
///     Ok(())
/// }
/// ```
pub struct ProfileLoaderBuilder {
    cache_size: usize,
    timeout: Duration,
    user_agent: String,
}

impl Default for ProfileLoaderBuilder {
    fn default() -> Self {
        Self {
            cache_size: DEFAULT_CACHE_SIZE,
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
            user_agent: format!("hl7v2-rs/{}", env!("CARGO_PKG_VERSION")),
        }
    }
}

impl ProfileLoaderBuilder {
    /// Create a new builder with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the maximum number of profiles to keep in the cache.
    pub fn cache_size(mut self, size: usize) -> Self {
        self.cache_size = size;
        self
    }

    /// Set the timeout for remote profile requests.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the User-Agent header for remote profile requests.
    pub fn user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = user_agent.into();
        self
    }

    /// Build the [`ProfileLoader`].
    pub fn build(self) -> ProfileLoader {
        let client = reqwest::Client::builder()
            .timeout(self.timeout)
            .user_agent(self.user_agent)
            .build()
            .unwrap_or_default();

        ProfileLoader {
            cache: Arc::new(RwLock::new(LruCache::new(
                std::num::NonZeroUsize::new(self.cache_size)
                    .unwrap_or(std::num::NonZeroUsize::new(1).unwrap()),
            ))),
            client,
            timeout: self.timeout,
        }
    }
}

/// A loader for HL7 v2 profiles with remote loading and caching.
///
/// The loader supports loading profiles from:
/// - Local files (using `file://` scheme or plain paths)
/// - Remote URLs (using `http://` or `https://` schemes)
///
/// Profiles are cached in memory using an LRU (Least Recently Used) cache.
/// For remote profiles, the loader uses ETag-based conditional requests
/// to minimize bandwidth and processing.
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
///     // Load from local file
///     let result = loader.load("profiles/adt_a01.yaml").await?;
///     println!("Loaded profile: {}", result.profile.message_structure);
///     
///     // Load from remote URL
///     let result = loader.load("https://example.com/hl7/profiles/oru_r01.yaml").await?;
///     println!("Loaded from URL, from cache: {}", result.from_cache);
///     
///     Ok(())
/// }
/// ```
pub struct ProfileLoader {
    cache: Arc<RwLock<LruCache<String, CacheEntry>>>,
    client: reqwest::Client,
    timeout: Duration,
}

impl Default for ProfileLoader {
    fn default() -> Self {
        Self::builder().build()
    }
}

impl ProfileLoader {
    /// Create a new [`ProfileLoader`] with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new [`ProfileLoaderBuilder`].
    pub fn builder() -> ProfileLoaderBuilder {
        ProfileLoaderBuilder::new()
    }

    /// Create a new [`ProfileLoader`] with specified options.
    pub fn with_options(cache_size: usize, timeout: Duration) -> Self {
        Self::builder()
            .cache_size(cache_size)
            .timeout(timeout)
            .build()
    }

    /// Set the timeout for remote profile requests.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self.client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .unwrap_or_default();
        self
    }

    /// Set the maximum number of profiles to keep in the cache.
    pub fn with_cache_size(self, size: usize) -> Self {
        let new_cache = LruCache::new(
            std::num::NonZeroUsize::new(size).unwrap_or(std::num::NonZeroUsize::new(1).unwrap()),
        );
        Self {
            cache: Arc::new(RwLock::new(new_cache)),
            client: self.client,
            timeout: self.timeout,
        }
    }

    /// Load a profile from a source (file path or URL).
    ///
    /// This is the main entry point for loading profiles. It automatically
    /// determines the source type and handles caching.
    pub async fn load(&self, source: &str) -> Result<LoadResult, ProfileLoadError> {
        if source.starts_with("http://") || source.starts_with("https://") {
            self.load_from_url(source).await
        } else {
            let path = source.strip_prefix("file://").unwrap_or(source);
            self.load_from_file(path).await
        }
    }

    /// Load a profile from a remote URL.
    pub async fn load_from_url(&self, url: &str) -> Result<LoadResult, ProfileLoadError> {
        // Check cache first
        let etag = {
            let mut cache = self.cache.write().await;
            cache.get(url).and_then(|e| e.etag.clone())
        };

        // Prepare request
        let mut request = self.client.get(url);
        if let Some(etag_val) = etag {
            request = request.header(reqwest::header::IF_NONE_MATCH, etag_val);
        }

        // Execute request
        let response = request.send().await?;

        // Handle response
        if response.status() == reqwest::StatusCode::NOT_MODIFIED {
            // Profile hasn't changed, return from cache
            let mut cache = self.cache.write().await;
            if let Some(entry) = cache.get(url) {
                return Ok(LoadResult {
                    profile: entry.profile.clone(),
                    from_cache: true,
                    etag: entry.etag.clone(),
                });
            }
        }

        if !response.status().is_success() {
            return Err(ProfileLoadError::Network(
                response.error_for_status().unwrap_err(),
            ));
        }

        // Get new ETag and content
        let new_etag = response
            .headers()
            .get(reqwest::header::ETAG)
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        let content = response.text().await?;

        // Parse profile
        let profile = load_profile(&content)?;

        // Update cache
        {
            let mut cache = self.cache.write().await;
            cache.put(
                url.to_string(),
                CacheEntry {
                    profile: profile.clone(),
                    etag: new_etag.clone(),
                    raw_content: content,
                },
            );
        }

        Ok(LoadResult {
            profile,
            from_cache: false,
            etag: new_etag,
        })
    }

    /// Load a profile from a local file.
    pub async fn load_from_file(&self, path: &str) -> Result<LoadResult, ProfileLoadError> {
        // For local files, we don't currently use the ETag logic,
        // but we still cache them by path to avoid re-parsing.

        {
            let mut cache = self.cache.write().await;
            if let Some(entry) = cache.get(path) {
                return Ok(LoadResult {
                    profile: entry.profile.clone(),
                    from_cache: true,
                    etag: None,
                });
            }
        }

        // Read file
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| ProfileLoadError::Io(e.to_string()))?;

        // Parse profile
        let profile = load_profile(&content)?;

        // Update cache
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

    /// Load a profile from a file synchronously.
    ///
    /// This bypasses the async loader and cache.
    pub fn load_file_sync(path: &str) -> Result<Profile, ProfileLoadError> {
        let content =
            std::fs::read_to_string(path).map_err(|e| ProfileLoadError::Io(e.to_string()))?;
        load_profile(&content)
    }

    /// Check if a profile is currently in the cache.
    pub async fn is_cached(&self, source: &str) -> bool {
        let cache = self.cache.read().await;
        // LruCache::contains is non-mutating and doesn't affect LRU order
        cache.contains(source)
    }

    /// Invalidate a profile in the cache.
    pub async fn invalidate(&self, source: &str) -> bool {
        let mut cache = self.cache.write().await;
        cache.pop(source).is_some()
    }

    /// Clear the profile cache.
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// Get the current number of profiles in the cache.
    pub async fn cache_size(&self) -> usize {
        let cache = self.cache.read().await;
        cache.len()
    }

    /// Prefetch a profile into the cache.
    pub async fn prefetch(&self, source: &str) -> Result<(), ProfileLoadError> {
        self.load(source).await?;
        Ok(())
    }

    /// Prefetch multiple profiles into the cache.
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

/// Global convenience function to load a profile from a URL.
pub async fn load_from_url(url: &str) -> Result<Profile, ProfileLoadError> {
    let loader = ProfileLoader::new();
    let result = loader.load_from_url(url).await?;
    Ok(result.profile)
}

/// Global convenience function to load a profile from a file.
pub fn load_from_file(path: &str) -> Result<Profile, ProfileLoadError> {
    ProfileLoader::load_file_sync(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_load_from_url() {
        let server = MockServer::start().await;
        let profile_yaml = "message_structure: ADT_A01\nversion: '2.5'\nsegments: []";

        Mock::given(method("GET"))
            .and(path("/profile.yaml"))
            .respond_with(ResponseTemplate::new(200).set_body_string(profile_yaml))
            .mount(&server)
            .await;

        let loader = ProfileLoader::new();
        let url = format!("{}/profile.yaml", server.uri());
        let result = loader.load(&url).await.unwrap();

        assert_eq!(result.profile.message_structure, "ADT_A01");
        assert!(!result.from_cache);
    }

    #[tokio::test]
    async fn test_cache_invalidation() {
        let server = MockServer::start().await;
        let profile_yaml = "message_structure: ADT_A01\nversion: '2.5'\nsegments: []";

        Mock::given(method("GET"))
            .and(path("/profile.yaml"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(profile_yaml)
                    .insert_header("ETag", "v1"),
            )
            .mount(&server)
            .await;

        let loader = ProfileLoader::new();
        let url = format!("{}/profile.yaml", server.uri());

        // First load
        let _ = loader.load(&url).await.unwrap();

        // Second load (should be from cache if we don't have conditional request logic yet,
        // or should use ETag)
        // Let's mock the 304 response
        server.reset().await;
        Mock::given(method("GET"))
            .and(path("/profile.yaml"))
            .respond_with(ResponseTemplate::new(304))
            .mount(&server)
            .await;

        let result = loader.load(&url).await.unwrap();
        assert!(result.from_cache);
    }

    #[tokio::test]
    async fn test_load_local_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("profile.yaml");
        let profile_yaml = "message_structure: ORU_R01\nversion: '2.5'\nsegments: []";
        std::fs::write(&file_path, profile_yaml).unwrap();

        let loader = ProfileLoader::new();
        let path_str = file_path.to_str().unwrap();
        let result = loader.load(path_str).await.unwrap();

        assert_eq!(result.profile.message_structure, "ORU_R01");
    }

    #[tokio::test]
    async fn test_invalid_url_scheme() {
        let loader = ProfileLoader::new();
        // Since we treat everything not starting with http as a file path,
        // this will fail with a file error, not a scheme error unless we explicitly check
        let result = loader.load("ftp://example.com/profile.yaml").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_file_not_found() {
        let loader = ProfileLoader::new();
        let result = loader.load("non_existent_file.yaml").await;
        assert!(result.is_err());
        assert!(matches!(result, Err(ProfileLoadError::Io(_))));
    }

    #[tokio::test]
    async fn test_parse_error() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("invalid.yaml");
        let invalid_yaml = "[: invalid yaml";
        std::fs::write(&file_path, invalid_yaml).unwrap();

        let loader = ProfileLoader::new();
        let result = loader.load(file_path.to_str().unwrap()).await;
        assert!(result.is_err());
        if let Err(ProfileLoadError::YamlParse(_)) = result {
            // expected
        } else {
            panic!("Expected YamlParse error, got {:?}", result);
        }
    }

    #[tokio::test]
    async fn test_lru_eviction() {
        let loader = ProfileLoader::builder().cache_size(1).build();

        // Create two temp files
        let temp_dir = tempfile::tempdir().unwrap();
        let p1 = temp_dir.path().join("p1.yaml");
        let p2 = temp_dir.path().join("p2.yaml");
        let yaml = "message_structure: ADT_A01\nversion: '2.5'\nsegments: []";
        std::fs::write(&p1, yaml).unwrap();
        std::fs::write(&p2, yaml).unwrap();

        loader.load(p1.to_str().unwrap()).await.unwrap();
        loader.load(p2.to_str().unwrap()).await.unwrap();

        // p1 should be evicted now
        let result = loader.load(p1.to_str().unwrap()).await.unwrap();
        assert!(!result.from_cache);
    }

    #[tokio::test]
    async fn test_clear_cache() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("profile.yaml");
        let yaml = "message_structure: ADT_A01\nversion: '2.5'\nsegments: []";
        std::fs::write(&file_path, yaml).unwrap();

        let loader = ProfileLoader::new();
        let path = file_path.to_str().unwrap();

        loader.load(path).await.unwrap();
        loader.clear_cache().await;

        let result = loader.load(path).await.unwrap();
        assert!(!result.from_cache);
    }
}
