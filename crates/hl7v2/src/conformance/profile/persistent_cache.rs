//! Persistent profile cache with PostgreSQL backend.

#![expect(
    clippy::missing_panics_doc,
    clippy::unwrap_used,
    reason = "Pre-existing persistent cache capacity and fixture unwraps moved from hl7v2-prof; cleanup is separate from this behavior-preserving module collapse."
)]

use super::{Profile, ProfileLoadError};
use async_lock::RwLock;
use lru::LruCache;
use std::num::NonZeroUsize;
use std::sync::Arc;

/// Two-tier profile cache: L1 (LRU in-memory), L2 (PostgreSQL)
pub struct PersistentProfileCache {
    l1_cache: Arc<RwLock<LruCache<String, Profile>>>,
    // In a real implementation, we would have a SQLx connection pool here
    // pool: sqlx::PgPool,
}

impl PersistentProfileCache {
    /// Create a new persistent cache
    pub fn new(capacity: usize) -> Self {
        Self {
            l1_cache: Arc::new(RwLock::new(LruCache::new(
                NonZeroUsize::new(capacity).unwrap(),
            ))),
        }
    }

    /// Get a profile from the cache
    pub async fn get(&self, name: &str) -> Option<Profile> {
        // Try L1 cache first
        let mut l1 = self.l1_cache.write().await;
        if let Some(profile) = l1.get(name) {
            return Some(profile.clone());
        }

        // In a real implementation, we would try L2 (Postgres) here
        None
    }

    /// Add a profile to the cache
    pub async fn put(&self, name: String, profile: Profile) -> Result<(), ProfileLoadError> {
        // Update L1 cache
        let mut l1 = self.l1_cache.write().await;
        l1.put(name, profile);

        // In a real implementation, we would also update L2 (Postgres) here
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Profile;
    use super::*;

    #[tokio::test]
    async fn test_cache_hit_miss() {
        let cache = PersistentProfileCache::new(2);
        let profile = Profile::default();

        // Miss
        assert!(cache.get("missing").await.is_none());

        // Put and Hit
        cache.put("test".to_string(), profile).await.unwrap();
        assert!(cache.get("test").await.is_some());
    }

    #[tokio::test]
    async fn test_cache_eviction() {
        let cache = PersistentProfileCache::new(2);
        let p1 = Profile::default();
        let p2 = Profile::default();
        let p3 = Profile::default();

        cache.put("p1".to_string(), p1).await.unwrap();
        cache.put("p2".to_string(), p2).await.unwrap();

        // Both should be in
        assert!(cache.get("p1").await.is_some());
        assert!(cache.get("p2").await.is_some());

        // Put third, should evict p1 (LRU)
        cache.put("p3".to_string(), p3).await.unwrap();

        assert!(cache.get("p3").await.is_some());
        assert!(cache.get("p2").await.is_some());
        assert!(cache.get("p1").await.is_none());
    }
}
