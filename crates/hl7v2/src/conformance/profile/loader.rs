//! Profile loader with remote loading and caching support.
//!
//! This module provides the [`ProfileLoader`](crate::conformance::profile::loader::ProfileLoader) struct for loading HL7 v2 profiles
//! from local files or remote URLs with ETag-based caching.

#![expect(
    clippy::unwrap_used,
    reason = "Pre-existing profile loader panic-family debt moved from hl7v2-prof; cleanup is separate from this behavior-preserving module collapse."
)]

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_lock::{Mutex, RwLock};
use lru::LruCache;

pub use super::ProfileLoadError;
use super::{Profile, load_profile};

/// Default cache size (number of profiles)
const DEFAULT_CACHE_SIZE: usize = 100;

/// Default request timeout in seconds
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Cache entry containing the profile and its ETag.
#[derive(Debug)]
struct CacheEntry {
    /// The loaded profile
    profile: Profile,
    /// ETag for conditional requests (if available)
    etag: Option<String>,
    /// Raw YAML content for comparison (kept for potential future use)
    #[expect(
        dead_code,
        reason = "Raw profile content is retained for compatibility with the existing loader cache shape."
    )]
    raw_content: String,
}

/// Profile cache storage with concurrent entry reads and serialized LRU updates.
///
/// `LruCache::get` promotes a hit and therefore requires mutable access. Keeping
/// the entries separate from the recency index lets cache hits clone their value
/// under a shared read lock while limiting exclusive access to the small index
/// update. The public loader API still observes normal LRU promotion semantics.
#[derive(Debug)]
struct ProfileCache {
    entries: RwLock<HashMap<String, CacheEntry>>,
    recency: Mutex<LruCache<String, ()>>,
}

impl ProfileCache {
    fn new(capacity: usize) -> Self {
        let capacity = std::num::NonZeroUsize::new(capacity)
            .unwrap_or(std::num::NonZeroUsize::new(1).unwrap());
        Self {
            entries: RwLock::new(HashMap::new()),
            recency: Mutex::new(LruCache::new(capacity)),
        }
    }

    async fn get_with<T>(&self, key: &str, project: impl FnOnce(&CacheEntry) -> T) -> Option<T> {
        // Check and promote in the recency index first. Writers hold this
        // same lock while updating both structures, so cancellation cannot
        // leave a partial mutation. An eviction can still race after this
        // guard is dropped; a missing entry then falls back to a reload.
        let present = {
            let mut recency = self.recency.lock().await;
            recency.get(key).is_some()
        };
        if !present {
            return None;
        }

        self.entries.read().await.get(key).map(project)
    }

    async fn insert(&self, key: String, entry: CacheEntry) {
        let mut recency = self.recency.lock().await;
        let mut entries = self.entries.write().await;
        let evicted_key = if recency.contains(&key) {
            None
        } else if recency.len() == recency.cap().get() {
            recency.pop_lru().map(|(evicted, _)| evicted)
        } else {
            None
        };
        recency.put(key.clone(), ());

        entries.insert(key, entry);
        if let Some(evicted_key) = evicted_key {
            entries.remove(&evicted_key);
        }
    }

    async fn contains(&self, key: &str) -> bool {
        self.entries.read().await.contains_key(key)
    }

    async fn remove(&self, key: &str) -> Option<CacheEntry> {
        let mut recency = self.recency.lock().await;
        let mut entries = self.entries.write().await;
        recency.pop(key);
        entries.remove(key)
    }

    async fn clear(&self) {
        let mut recency = self.recency.lock().await;
        let mut entries = self.entries.write().await;
        recency.clear();
        entries.clear();
    }

    async fn len(&self) -> usize {
        self.entries.read().await.len()
    }
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
/// use hl7v2::conformance::profile::loader::ProfileLoader;
/// use hl7v2::conformance::profile::ProfileLoadError;
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
            cache: Arc::new(ProfileCache::new(self.cache_size)),
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
/// use hl7v2::conformance::profile::loader::{ProfileLoader, ProfileLoadError};
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
    cache: Arc<ProfileCache>,
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
        Self {
            cache: Arc::new(ProfileCache::new(size)),
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
        let etag = self
            .cache
            .get_with(url, |entry| entry.etag.clone())
            .await
            .flatten();

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
            if let Some((profile, etag)) = self
                .cache
                .get_with(url, |entry| (entry.profile.clone(), entry.etag.clone()))
                .await
            {
                return Ok(LoadResult {
                    profile,
                    from_cache: true,
                    etag,
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
            .map(std::string::ToString::to_string);

        let content = response.text().await?;

        // Parse profile
        let profile = load_profile(&content)?;

        // Update cache
        {
            self.cache
                .insert(
                    url.to_string(),
                    CacheEntry {
                        profile: profile.clone(),
                        etag: new_etag.clone(),
                        raw_content: content,
                    },
                )
                .await;
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
            if let Some(profile) = self
                .cache
                .get_with(path, |entry| entry.profile.clone())
                .await
            {
                return Ok(LoadResult {
                    profile,
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
            self.cache
                .insert(
                    path.to_string(),
                    CacheEntry {
                        profile: profile.clone(),
                        etag: None,
                        raw_content: content,
                    },
                )
                .await;
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
        self.cache.contains(source).await
    }

    /// Invalidate a profile in the cache.
    pub async fn invalidate(&self, source: &str) -> bool {
        self.cache.remove(source).await.is_some()
    }

    /// Clear the profile cache.
    pub async fn clear_cache(&self) {
        self.cache.clear().await;
    }

    /// Get the current number of profiles in the cache.
    pub async fn cache_size(&self) -> usize {
        self.cache.len().await
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
    #![expect(
        clippy::assertions_on_result_states,
        clippy::panic,
        reason = "Pre-existing profile loader test debt moved from hl7v2-prof; cleanup is separate from this behavior-preserving module collapse."
    )]

    use super::*;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn profile_loader_without_proxy() -> ProfileLoader {
        let cache_size = std::num::NonZeroUsize::new(DEFAULT_CACHE_SIZE)
            .unwrap_or(std::num::NonZeroUsize::new(1).unwrap());

        ProfileLoader {
            cache: Arc::new(ProfileCache::new(cache_size.get())),
            client: reqwest::Client::builder()
                .no_proxy()
                .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
                .build()
                .unwrap_or_default(),
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
        }
    }

    #[tokio::test]
    async fn test_load_from_url() {
        let server = MockServer::start().await;
        let profile_yaml = "message_structure: ADT_A01\nversion: '2.5'\nsegments: []";

        Mock::given(method("GET"))
            .and(path("/profile.yaml"))
            .respond_with(ResponseTemplate::new(200).set_body_string(profile_yaml))
            .mount(&server)
            .await;

        let loader = profile_loader_without_proxy();
        let url = format!("{}/profile.yaml", server.uri());
        let result = loader.load(&url).await.unwrap();

        assert_eq!(result.profile.message_structure, "ADT_A01");
        assert!(!result.from_cache);
        assert_eq!(result.etag, None);
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

        let loader = profile_loader_without_proxy();
        let url = format!("{}/profile.yaml", server.uri());

        // First load populates the cache with the response ETag.
        let first = loader.load(&url).await.unwrap();
        assert!(!first.from_cache);
        assert_eq!(first.etag.as_deref(), Some("v1"));

        // Second load should revalidate with If-None-Match and satisfy 304s from cache.
        server.reset().await;
        Mock::given(method("GET"))
            .and(path("/profile.yaml"))
            .and(header("if-none-match", "v1"))
            .respond_with(ResponseTemplate::new(304))
            .mount(&server)
            .await;

        let result = loader.load(&url).await.unwrap();
        assert!(result.from_cache);
        assert_eq!(result.profile.message_structure, "ADT_A01");
        assert_eq!(result.etag.as_deref(), Some("v1"));
    }

    #[tokio::test]
    async fn test_load_from_url_updates_cache_when_remote_profile_changes() {
        let server = MockServer::start().await;
        let first_profile_yaml = "message_structure: ADT_A01\nversion: '2.5'\nsegments: []";
        let second_profile_yaml = "message_structure: ORU_R01\nversion: '2.5'\nsegments: []";

        Mock::given(method("GET"))
            .and(path("/profile.yaml"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(first_profile_yaml)
                    .insert_header("ETag", "v1"),
            )
            .mount(&server)
            .await;

        let loader = profile_loader_without_proxy();
        let url = format!("{}/profile.yaml", server.uri());
        let first = loader.load(&url).await.unwrap();
        assert_eq!(first.profile.message_structure, "ADT_A01");
        assert_eq!(first.etag.as_deref(), Some("v1"));

        server.reset().await;
        Mock::given(method("GET"))
            .and(path("/profile.yaml"))
            .and(header("if-none-match", "v1"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(second_profile_yaml)
                    .insert_header("ETag", "v2"),
            )
            .mount(&server)
            .await;

        let second = loader.load(&url).await.unwrap();
        assert!(!second.from_cache);
        assert_eq!(second.profile.message_structure, "ORU_R01");
        assert_eq!(second.etag.as_deref(), Some("v2"));
    }

    #[tokio::test]
    async fn test_file_scheme_loads_and_reuses_local_file_cache() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("profile.yaml");
        let profile_yaml = "message_structure: PPR_PC1\nversion: '2.5'\nsegments: []";
        std::fs::write(&file_path, profile_yaml).unwrap();

        let loader = ProfileLoader::new();
        let path = file_path.to_str().unwrap();
        let file_url = format!("file://{path}");

        let first = loader.load(&file_url).await.unwrap();
        assert_eq!(first.profile.message_structure, "PPR_PC1");
        assert!(!first.from_cache);
        assert!(loader.is_cached(path).await);
        assert!(!loader.is_cached(&file_url).await);

        let second = loader.load(&file_url).await.unwrap();
        assert!(second.from_cache);
        assert_eq!(second.profile.message_structure, "PPR_PC1");
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
    async fn test_lru_cache_hit_promotes_entry() {
        let loader = ProfileLoader::builder().cache_size(2).build();
        let temp_dir = tempfile::tempdir().unwrap();
        let p1 = temp_dir.path().join("p1.yaml");
        let p2 = temp_dir.path().join("p2.yaml");
        let p3 = temp_dir.path().join("p3.yaml");
        let yaml = "message_structure: ADT_A01\nversion: '2.5'\nsegments: []";
        std::fs::write(&p1, yaml).unwrap();
        std::fs::write(&p2, yaml).unwrap();
        std::fs::write(&p3, yaml).unwrap();

        loader.load(p1.to_str().unwrap()).await.unwrap();
        loader.load(p2.to_str().unwrap()).await.unwrap();
        assert!(loader.load(p1.to_str().unwrap()).await.unwrap().from_cache);
        loader.load(p3.to_str().unwrap()).await.unwrap();

        assert!(loader.load(p1.to_str().unwrap()).await.unwrap().from_cache);
        assert!(!loader.load(p2.to_str().unwrap()).await.unwrap().from_cache);
    }

    #[tokio::test]
    async fn cancelled_insert_keeps_entry_and_recency_indexes_consistent() {
        let cache = Arc::new(ProfileCache::new(2));
        let profile =
            load_profile("message_structure: ADT_A01\nversion: '2.5'\nsegments: []").unwrap();
        let entry = CacheEntry {
            profile,
            etag: None,
            raw_content: String::new(),
        };
        let entries_guard = cache.entries.write().await;
        let pending_cache = Arc::clone(&cache);
        let task = tokio::spawn(async move {
            pending_cache.insert("cancelled".to_owned(), entry).await;
        });

        for _ in 0..10 {
            tokio::task::yield_now().await;
        }
        task.abort();
        assert!(task.await.is_err());
        drop(entries_guard);

        assert_eq!(cache.len().await, 0);
        assert!(!cache.recency.lock().await.contains("cancelled"));
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
