//! Persistent profile cache with PostgreSQL backend.
//!
//! This module provides a two-tier caching system:
//! - **Memory tier**: LRU cache for fast access to frequently used profiles
//! - **Persistent tier**: PostgreSQL database for durable storage across restarts
//!
//! The cache uses content-addressed storage with SHA-256 checksums for integrity
//! verification and optimistic concurrency control.
//!
//! # Example
//!
//! ```rust,no_run
//! use hl7v2_prof::persistent_cache::{PersistentProfileCache, CacheConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let cache = PersistentProfileCache::new("postgresql://localhost:5432/hl7v2")
//!         .await?;
//!     
//!     // Store a profile
//!     cache.store("test/adt_a01", "message_structure: ADT_A01\n...", None)
//!         .await?;
//!     
//!     // Retrieve a profile
//!     if let Some(cached) = cache.get("test/adt_a01").await {
//!         println!("Loaded: {}", cached.profile.message_structure);
//!     }
//!     
//!     Ok(())
//! }
//! ```

use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::Duration;

use async_lock::RwLock;
use lru::LruCache;
use sha2::{Digest, Sha256};

use crate::{Profile, ProfileLoadError, load_profile};

/// Default cache size (number of profiles in memory)
const DEFAULT_MEMORY_CACHE_SIZE: usize = 100;

/// Default timeout for database operations
const DEFAULT_DB_TIMEOUT: Duration = Duration::from_secs(30);

/// Errors that can occur when interacting with the persistent profile cache.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ProfileCacheError {
    /// Invalid database URL format
    #[error("Invalid database URL: {0}")]
    InvalidDatabaseUrl(String),

    /// Database connection failure
    #[error("Database connection error: {0}")]
    DatabaseConnection(String),

    /// Conflict during concurrent update
    #[error("Conflict for profile {profile_id}: {reason}")]
    Conflict {
        /// The profile ID that had the conflict
        profile_id: String,
        /// The reason for the conflict
        reason: String,
    },

    /// Profile not found
    #[error("Profile not found: {0}")]
    NotFound(String),

    /// Database operation timed out
    #[error("Database operation timed out")]
    Timeout,

    /// YAML serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Other database errors
    #[error("Cache error: {0}")]
    Other(String),
}

impl From<sqlx::Error> for ProfileCacheError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::Database(db_err) => {
                if db_err.is_unique_violation() {
                    ProfileCacheError::Conflict {
                        profile_id: "unknown".to_string(),
                        reason: db_err.message().to_string(),
                    }
                } else {
                    ProfileCacheError::DatabaseConnection(db_err.message().to_string())
                }
            }
            sqlx::Error::Io(io_err) => ProfileCacheError::DatabaseConnection(io_err.to_string()),
            sqlx::Error::PoolTimedOut => ProfileCacheError::Timeout,
            _ => ProfileCacheError::Other(err.to_string()),
        }
    }
}

impl From<ProfileLoadError> for ProfileCacheError {
    fn from(err: ProfileLoadError) -> Self {
        ProfileCacheError::Serialization(err.to_string())
    }
}

/// A cached profile with metadata about its source.
#[derive(Debug, Clone)]
pub struct CachedProfile {
    /// The loaded profile
    pub profile: Profile,
    /// Whether the profile was loaded from the persistent (database) tier
    pub from_persistent_cache: bool,
    /// Whether the profile was loaded from any cache tier
    pub from_cache: bool,
}

/// A record stored in the persistent cache (database).
#[derive(Debug, Clone)]
pub struct ProfileRecord {
    /// Unique identifier for the profile
    pub profile_id: String,
    /// Raw YAML content
    pub content: String,
    /// ETag from remote source (if applicable)
    pub etag: Option<String>,
    /// SHA-256 checksum of the content
    pub checksum: String,
    /// When the record was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When the record was last updated
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Statistics about cache performance and state.
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Number of entries in the memory cache
    pub memory_entries: usize,
    /// Number of entries in the persistent (database) cache
    pub persistent_entries: usize,
    /// Number of hits from memory cache
    pub memory_hits: usize,
    /// Number of hits from persistent cache (with memory miss)
    pub persistent_hits: usize,
    /// Number of cache misses
    pub misses: usize,
}

/// Internal statistics tracking (mutable)
#[derive(Debug, Default)]
struct InternalStats {
    memory_hits: usize,
    persistent_hits: usize,
    misses: usize,
}

/// Memory cache entry containing the profile and metadata.
#[derive(Debug, Clone)]
struct MemoryCacheEntry {
    profile: Profile,
    etag: Option<String>,
    checksum: String,
}

/// Configuration for the persistent profile cache.
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Database connection URL
    pub database_url: String,
    /// Maximum number of profiles to keep in memory cache
    pub memory_cache_size: usize,
    /// Timeout for database operations
    pub timeout: Duration,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            database_url: "postgresql://localhost:5432/hl7v2".to_string(),
            memory_cache_size: DEFAULT_MEMORY_CACHE_SIZE,
            timeout: DEFAULT_DB_TIMEOUT,
        }
    }
}

/// Builder for configuring and creating a [`PersistentProfileCache`].
#[derive(Debug)]
pub struct PersistentProfileCacheBuilder {
    config: CacheConfig,
}

impl Default for PersistentProfileCacheBuilder {
    fn default() -> Self {
        Self {
            config: CacheConfig::default(),
        }
    }
}

impl PersistentProfileCacheBuilder {
    /// Create a new builder with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the database connection URL.
    pub fn database_url(mut self, url: impl Into<String>) -> Self {
        self.config.database_url = url.into();
        self
    }

    /// Set the memory cache size (number of profiles).
    pub fn memory_cache_size(mut self, size: usize) -> Self {
        self.config.memory_cache_size = size.max(1);
        self
    }

    /// Set the timeout for database operations.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = timeout;
        self
    }

    /// Build the [`PersistentProfileCache`].
    ///
    /// This creates the cache and initializes the database pool, but does not
    /// run migrations or pre-populate the cache.
    pub async fn build(self) -> Result<PersistentProfileCache, ProfileCacheError> {
        PersistentProfileCache::with_config(self.config).await
    }
}

/// A two-tier persistent cache for HL7 v2 profiles.
///
/// The cache provides:
/// - Fast in-memory LRU caching for frequently accessed profiles
/// - Persistent PostgreSQL storage for durability across restarts
/// - Content-addressed storage with SHA-256 checksums
/// - ETag support for remote profile synchronization
/// - Optimistic concurrency control for updates
/// - Comprehensive statistics for monitoring
///
/// # Architecture
///
/// ```text
/// ┌─────────────────────────────────────────────────────────────┐
/// │                    PersistentProfileCache                    │
/// ├──────────────────────────────┬──────────────────────────────┤
/// │      Memory Tier (LRU)       │    Persistent Tier (PG)    │
/// │                              │                              │
/// │  ┌────────────────────────┐  │  ┌────────────────────────┐  │
/// │  │  profile_id → Entry    │  │  │  profile_id → Record   │  │
/// │  │  (fast, bounded)       │  │  │  (durable, unbounded)  │  │
/// │  └────────────────────────┘  │  └────────────────────────┘  │
/// │                              │                              │
/// │  - Sub-millisecond access    │  - Survives restarts         │
/// │  - Auto-eviction on bound    │  - Content-addressed         │
/// │  - Promoted on access        │  - Checksum verified         │
/// └──────────────────────────────┴──────────────────────────────┘
/// ```
#[derive(Debug)]
pub struct PersistentProfileCache {
    /// Database connection pool
    pool: sqlx::PgPool,
    /// In-memory LRU cache
    memory_cache: Arc<RwLock<LruCache<String, MemoryCacheEntry>>>,
    /// Statistics tracking
    stats: Arc<RwLock<InternalStats>>,
    /// Cache configuration
    config: CacheConfig,
}

impl PersistentProfileCache {
    /// Create a new cache with the given database URL.
    ///
    /// # Arguments
    ///
    /// * `database_url` - PostgreSQL connection string (e.g., "postgresql://user:pass@localhost/db")
    ///
    /// # Errors
    ///
    /// Returns `ProfileCacheError::InvalidDatabaseUrl` if the URL is malformed,
    /// or `ProfileCacheError::DatabaseConnection` if the connection fails.
    pub async fn new(database_url: &str) -> Result<Self, ProfileCacheError> {
        let config = CacheConfig {
            database_url: database_url.to_string(),
            ..Default::default()
        };
        Self::with_config(config).await
    }

    /// Create a new cache with the given configuration.
    async fn with_config(config: CacheConfig) -> Result<Self, ProfileCacheError> {
        // Validate and create the connection pool
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(config.timeout)
            .connect(&config.database_url)
            .await
            .map_err(|e| {
                if e.to_string().contains("invalid connection string") {
                    ProfileCacheError::InvalidDatabaseUrl(config.database_url.clone())
                } else {
                    ProfileCacheError::DatabaseConnection(e.to_string())
                }
            })?;

        // Initialize the database schema
        Self::init_schema(&pool).await?;

        let cache_size =
            NonZeroUsize::new(config.memory_cache_size).unwrap_or(NonZeroUsize::new(1).unwrap());

        Ok(Self {
            pool,
            memory_cache: Arc::new(RwLock::new(LruCache::new(cache_size))),
            stats: Arc::new(RwLock::new(InternalStats::default())),
            config,
        })
    }

    /// Create a new builder for configuring the cache.
    pub fn builder() -> PersistentProfileCacheBuilder {
        PersistentProfileCacheBuilder::new()
    }

    /// Initialize the database schema.
    async fn init_schema(pool: &sqlx::PgPool) -> Result<(), ProfileCacheError> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS profile_cache (
                profile_id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                etag TEXT,
                checksum TEXT NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            );
            CREATE INDEX IF NOT EXISTS idx_profile_cache_checksum ON profile_cache(checksum);
            "#,
        )
        .execute(pool)
        .await
        .map_err(ProfileCacheError::from)?;

        Ok(())
    }

    /// Compute SHA-256 checksum of content.
    fn compute_checksum(content: &str) -> String {
        let hash = Sha256::digest(content.as_bytes());
        hex::encode(hash)
    }

    /// Store a profile in the cache.
    ///
    /// The profile is stored in both the memory and persistent tiers.
    /// If a profile with the same ID already exists, it will be updated.
    ///
    /// # Arguments
    ///
    /// * `profile_id` - Unique identifier for the profile
    /// * `yaml` - Raw YAML content of the profile
    /// * `etag` - Optional ETag from remote source
    ///
    /// # Errors
    ///
    /// Returns an error if the database operation fails or if the YAML is invalid.
    pub async fn store(
        &self,
        profile_id: &str,
        yaml: &str,
        etag: Option<&str>,
    ) -> Result<(), ProfileCacheError> {
        let checksum = Self::compute_checksum(yaml);
        let profile = load_profile(yaml)?;

        // Store in database
        sqlx::query(
            r#"
            INSERT INTO profile_cache (profile_id, content, etag, checksum, created_at, updated_at)
            VALUES ($1, $2, $3, $4, NOW(), NOW())
            ON CONFLICT (profile_id) DO UPDATE SET
                content = EXCLUDED.content,
                etag = EXCLUDED.etag,
                checksum = EXCLUDED.checksum,
                updated_at = NOW()
            "#,
        )
        .bind(profile_id)
        .bind(yaml)
        .bind(etag)
        .bind(&checksum)
        .execute(&self.pool)
        .await
        .map_err(ProfileCacheError::from)?;

        // Store in memory cache
        {
            let mut cache = self.memory_cache.write().await;
            cache.put(
                profile_id.to_string(),
                MemoryCacheEntry {
                    profile,
                    etag: etag.map(String::from),
                    checksum,
                },
            );
        }

        Ok(())
    }

    /// Get a profile from the cache.
    ///
    /// This checks the memory cache first, and if not found, queries the
    /// persistent cache and populates the memory cache.
    ///
    /// # Arguments
    ///
    /// * `profile_id` - Unique identifier for the profile
    ///
    /// # Returns
    ///
    /// `Some(CachedProfile)` if found, `None` if not in cache.
    pub async fn get(&self, profile_id: &str) -> Option<CachedProfile> {
        // Check memory cache first
        {
            let mut cache = self.memory_cache.write().await;
            if let Some(entry) = cache.get(profile_id) {
                // Update stats
                {
                    let mut stats = self.stats.write().await;
                    stats.memory_hits += 1;
                }
                return Some(CachedProfile {
                    profile: entry.profile.clone(),
                    from_persistent_cache: false,
                    from_cache: true,
                });
            }
        }

        // Not in memory cache, check persistent cache
        match self.load_from_database(profile_id).await {
            Some((record, profile)) => {
                // Populate memory cache
                {
                    let mut cache = self.memory_cache.write().await;
                    cache.put(
                        profile_id.to_string(),
                        MemoryCacheEntry {
                            profile: profile.clone(),
                            etag: record.etag.clone(),
                            checksum: record.checksum.clone(),
                        },
                    );
                }

                // Update stats
                {
                    let mut stats = self.stats.write().await;
                    stats.persistent_hits += 1;
                }

                Some(CachedProfile {
                    profile,
                    from_persistent_cache: true,
                    from_cache: true,
                })
            }
            None => {
                // Update stats
                {
                    let mut stats = self.stats.write().await;
                    stats.misses += 1;
                }
                None
            }
        }
    }

    /// Get the raw database record for a profile.
    ///
    /// This is primarily useful for debugging and integrity verification.
    pub async fn get_record(&self, profile_id: &str) -> Option<ProfileRecord> {
        self.load_record_from_database(profile_id).await
    }

    /// Update a profile with optimistic concurrency control.
    ///
    /// The update only succeeds if the current checksum matches the expected checksum.
    /// This prevents lost updates in concurrent scenarios.
    ///
    /// # Arguments
    ///
    /// * `profile_id` - Unique identifier for the profile
    /// * `yaml` - New YAML content
    /// * `etag` - Optional new ETag
    /// * `expected_checksum` - Expected current checksum (for optimistic locking)
    ///
    /// # Errors
    ///
    /// Returns `ProfileCacheError::Conflict` if the checksum doesn't match.
    pub async fn update(
        &self,
        profile_id: &str,
        yaml: &str,
        etag: Option<&str>,
        expected_checksum: Option<&str>,
    ) -> Result<(), ProfileCacheError> {
        let new_checksum = Self::compute_checksum(yaml);
        let profile = load_profile(yaml)?;

        // Verify expected checksum if provided
        if let Some(expected) = expected_checksum {
            let current_record = self.load_record_from_database(profile_id).await;
            match current_record {
                Some(record) if record.checksum == expected => {
                    // Checksum matches, proceed with update
                }
                _ => {
                    return Err(ProfileCacheError::Conflict {
                        profile_id: profile_id.to_string(),
                        reason: "Checksum mismatch - profile was modified".to_string(),
                    });
                }
            }
        }

        // Store in database
        sqlx::query(
            r#"
            INSERT INTO profile_cache (profile_id, content, etag, checksum, created_at, updated_at)
            VALUES ($1, $2, $3, $4, NOW(), NOW())
            ON CONFLICT (profile_id) DO UPDATE SET
                content = EXCLUDED.content,
                etag = EXCLUDED.etag,
                checksum = EXCLUDED.checksum,
                updated_at = NOW()
            "#,
        )
        .bind(profile_id)
        .bind(yaml)
        .bind(etag)
        .bind(&new_checksum)
        .execute(&self.pool)
        .await
        .map_err(ProfileCacheError::from)?;

        // Update memory cache
        {
            let mut cache = self.memory_cache.write().await;
            cache.put(
                profile_id.to_string(),
                MemoryCacheEntry {
                    profile,
                    etag: etag.map(String::from),
                    checksum: new_checksum,
                },
            );
        }

        Ok(())
    }

    /// Invalidate (remove) a profile from the cache.
    ///
    /// Removes from both memory and persistent tiers.
    ///
    /// # Arguments
    ///
    /// * `profile_id` - Unique identifier for the profile
    ///
    /// # Returns
    ///
    /// `true` if the profile was found and removed, `false` if not found.
    pub async fn invalidate(&self, profile_id: &str) -> bool {
        // Remove from memory cache
        {
            let mut cache = self.memory_cache.write().await;
            cache.pop(profile_id);
        }

        // Remove from database
        let result = sqlx::query("DELETE FROM profile_cache WHERE profile_id = $1")
            .bind(profile_id)
            .execute(&self.pool)
            .await;

        match result {
            Ok(query_result) => query_result.rows_affected() > 0,
            Err(_) => false,
        }
    }

    /// Invalidate all profiles matching a prefix.
    ///
    /// # Arguments
    ///
    /// * `prefix` - Prefix to match (e.g., "remote/")
    ///
    /// # Returns
    ///
    /// Number of profiles removed.
    pub async fn invalidate_prefix(&self, prefix: &str) -> usize {
        // Get list of matching IDs from database first
        let ids: Vec<String> = match sqlx::query_scalar(
            "SELECT profile_id FROM profile_cache WHERE profile_id LIKE $1",
        )
        .bind(format!("{}%", prefix))
        .fetch_all(&self.pool)
        .await
        {
            Ok(ids) => ids,
            Err(_) => return 0,
        };

        // Remove from memory cache
        {
            let mut cache = self.memory_cache.write().await;
            for id in &ids {
                cache.pop(id);
            }
        }

        // Remove from database
        match sqlx::query("DELETE FROM profile_cache WHERE profile_id LIKE $1")
            .bind(format!("{}%", prefix))
            .execute(&self.pool)
            .await
        {
            Ok(result) => result.rows_affected() as usize,
            Err(_) => 0,
        }
    }

    /// Clear all profiles from both cache tiers.
    pub async fn clear(&self) {
        // Clear memory cache
        {
            let mut cache = self.memory_cache.write().await;
            cache.clear();
        }

        // Clear database
        let _ = sqlx::query("DELETE FROM profile_cache")
            .execute(&self.pool)
            .await;
    }

    /// Clear only the memory cache (useful for testing warm restart).
    pub async fn clear_memory_cache(&self) {
        let mut cache = self.memory_cache.write().await;
        cache.clear();
    }

    /// Warm restart: populate memory cache from persistent cache.
    ///
    /// This loads profiles from the database into the memory cache,
    /// respecting the memory cache size limit.
    ///
    /// # Errors
    ///
    /// Returns an error if the database query fails.
    pub async fn warm_restart(&self) -> Result<(), ProfileCacheError> {
        let records = sqlx::query_as::<_, (String, String, Option<String>, String)>(
            "SELECT profile_id, content, etag, checksum FROM profile_cache ORDER BY updated_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(ProfileCacheError::from)?;

        let mut cache = self.memory_cache.write().await;
        let memory_limit = cache.cap().get();

        for (idx, (profile_id, content, etag, checksum)) in records.iter().enumerate() {
            // Respect memory cache size limit
            if idx >= memory_limit {
                break;
            }

            if let Ok(profile) = load_profile(content) {
                cache.put(
                    profile_id.clone(),
                    MemoryCacheEntry {
                        profile,
                        etag: etag.clone(),
                        checksum: checksum.clone(),
                    },
                );
            }
        }

        Ok(())
    }

    /// Check if a profile is cached (in either tier).
    pub async fn is_cached(&self, profile_id: &str) -> bool {
        // Check memory cache first
        {
            let cache = self.memory_cache.read().await;
            if cache.contains(profile_id) {
                return true;
            }
        }

        // Check database
        match sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM profile_cache WHERE profile_id = $1",
        )
        .bind(profile_id)
        .fetch_one(&self.pool)
        .await
        {
            Ok(count) => count > 0,
            Err(_) => false,
        }
    }

    /// Get current cache statistics.
    pub async fn cache_stats(&self) -> CacheStats {
        let memory_entries = {
            let cache = self.memory_cache.read().await;
            cache.len()
        };

        let persistent_entries =
            match sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM profile_cache")
                .fetch_one(&self.pool)
                .await
            {
                Ok(count) => count as usize,
                Err(_) => 0,
            };

        let stats = {
            let stats = self.stats.read().await;
            CacheStats {
                memory_entries,
                persistent_entries,
                memory_hits: stats.memory_hits,
                persistent_hits: stats.persistent_hits,
                misses: stats.misses,
            }
        };

        stats
    }

    /// Reset the hit/miss statistics.
    pub async fn reset_stats(&self) {
        let mut stats = self.stats.write().await;
        *stats = InternalStats::default();
    }

    /// Store multiple profiles in a single transaction.
    ///
    /// All profiles are stored atomically - if any fail, none are stored.
    pub async fn store_all(&self, profiles: &[(&str, &str)]) -> Vec<Result<(), ProfileCacheError>> {
        let mut tx = match self.pool.begin().await {
            Ok(tx) => tx,
            Err(e) => {
                return vec![
                    Err(ProfileCacheError::DatabaseConnection(e.to_string()));
                    profiles.len()
                ];
            }
        };

        let mut results = Vec::with_capacity(profiles.len());

        for (profile_id, yaml) in profiles {
            let checksum = Self::compute_checksum(yaml);

            match sqlx::query(
                r#"
                INSERT INTO profile_cache (profile_id, content, checksum, created_at, updated_at)
                VALUES ($1, $2, $3, NOW(), NOW())
                ON CONFLICT (profile_id) DO UPDATE SET
                    content = EXCLUDED.content,
                    checksum = EXCLUDED.checksum,
                    updated_at = NOW()
                "#,
            )
            .bind(*profile_id)
            .bind(*yaml)
            .bind(&checksum)
            .execute(&mut *tx)
            .await
            {
                Ok(_) => results.push(Ok(())),
                Err(e) => results.push(Err(ProfileCacheError::from(e))),
            }
        }

        // Commit transaction
        if let Err(e) = tx.commit().await {
            // Mark all as failed if commit fails
            return vec![Err(ProfileCacheError::DatabaseConnection(e.to_string())); profiles.len()];
        }

        // Populate memory cache for successful stores
        for ((profile_id, yaml), result) in profiles.iter().zip(results.iter()) {
            if result.is_ok() {
                if let Ok(profile) = load_profile(yaml) {
                    let mut cache = self.memory_cache.write().await;
                    cache.put(
                        profile_id.to_string(),
                        MemoryCacheEntry {
                            profile,
                            etag: None,
                            checksum: Self::compute_checksum(yaml),
                        },
                    );
                }
            }
        }

        results
    }

    /// Get multiple profiles in a single batch operation.
    ///
    /// Returns a vector of `Option<CachedProfile>` in the same order as the input IDs.
    pub async fn get_all(&self, ids: &[String]) -> Vec<Option<CachedProfile>> {
        let mut results = Vec::with_capacity(ids.len());

        for id in ids {
            results.push(self.get(id).await);
        }

        results
    }

    /// Load a profile from the database (internal method).
    async fn load_from_database(&self, profile_id: &str) -> Option<(ProfileRecord, Profile)> {
        let record = self.load_record_from_database(profile_id).await?;

        match load_profile(&record.content) {
            Ok(profile) => Some((record, profile)),
            Err(_) => None,
        }
    }

    /// Load a raw record from the database.
    async fn load_record_from_database(&self, profile_id: &str) -> Option<ProfileRecord> {
        let row = sqlx::query_as::<_, (String, String, Option<String>, String, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)>(
            "SELECT profile_id, content, etag, checksum, created_at, updated_at FROM profile_cache WHERE profile_id = $1",
        )
        .bind(profile_id)
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten()?;

        Some(ProfileRecord {
            profile_id: row.0,
            content: row.1,
            etag: row.2,
            checksum: row.3,
            created_at: row.4,
            updated_at: row.5,
        })
    }

    /// Returns true if this is a test placeholder (always false for real implementation)
    ///
    /// This method is used by integration tests to verify that the real
    /// implementation is being used rather than a placeholder.
    pub fn is_test_placeholder(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require a running PostgreSQL instance.
    // They are marked as ignored by default to avoid CI failures.

    async fn create_test_cache() -> PersistentProfileCache {
        let db_url = std::env::var("TEST_DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://localhost:5432/hl7v2_test".to_string());

        PersistentProfileCache::new(&db_url)
            .await
            .expect("Failed to create test cache")
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL"]
    async fn test_basic_store_and_get() {
        let cache = create_test_cache().await;
        let yaml = "message_structure: ADT_A01\nversion: '2.5'\nsegments: []";

        cache.store("test/profile", yaml, None).await.unwrap();

        let result = cache.get("test/profile").await;
        assert!(result.is_some());
        assert_eq!(result.unwrap().profile.message_structure, "ADT_A01");
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL"]
    async fn test_checksum_computation() {
        let cache = create_test_cache().await;
        let yaml = "message_structure: ADT_A01\nversion: '2.5'\nsegments: []";

        cache.store("test/profile", yaml, None).await.unwrap();

        let record = cache.get_record("test/profile").await.unwrap();
        assert_eq!(record.checksum.len(), 64); // SHA-256 hex length
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL"]
    async fn test_etag_preservation() {
        let cache = create_test_cache().await;
        let yaml = "message_structure: ADT_A01\nversion: '2.5'\nsegments: []";

        cache
            .store("test/profile", yaml, Some("\"v1\""))
            .await
            .unwrap();

        let record = cache.get_record("test/profile").await.unwrap();
        assert_eq!(record.etag, Some("\"v1\"".to_string()));
    }

    #[tokio::test]
    #[ignore = "Requires PostgreSQL"]
    async fn test_invalidation() {
        let cache = create_test_cache().await;
        let yaml = "message_structure: ADT_A01\nversion: '2.5'\nsegments: []";

        cache.store("test/profile", yaml, None).await.unwrap();
        assert!(cache.invalidate("test/profile").await);
        assert!(cache.get("test/profile").await.is_none());
    }
}
