use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use std::collections::HashMap;

/// Generic cache entry with timestamp
#[derive(Clone)]
struct CacheEntry<T> {
    data: T,
    timestamp: Instant,
}

impl<T> CacheEntry<T> {
    fn new(data: T) -> Self {
        Self {
            data,
            timestamp: Instant::now(),
        }
    }

    fn is_expired(&self, ttl: Duration) -> bool {
        self.timestamp.elapsed() > ttl
    }
}

/// Generic cache for any data type
pub struct DataCache<T: Clone> {
    cache: Arc<RwLock<HashMap<String, CacheEntry<T>>>>,
    ttl: Duration,
}

impl<T: Clone> DataCache<T> {
    pub fn new(ttl_seconds: u64) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            ttl: Duration::from_secs(ttl_seconds),
        }
    }

    /// Get cached value if available and not expired
    pub async fn get(&self, key: &str) -> Option<T> {
        let cache = self.cache.read().await;
        if let Some(entry) = cache.get(key) {
            if !entry.is_expired(self.ttl) {
                return Some(entry.data.clone());
            }
        }
        None
    }

    /// Store value in cache
    pub async fn set(&self, key: String, value: T) {
        let mut cache = self.cache.write().await;
        cache.insert(key, CacheEntry::new(value));
    }

    /// Get or fetch: returns cached value if available, otherwise calls fetch_fn and caches result
    pub async fn get_or_fetch<F, Fut, E>(
        &self,
        key: &str,
        fetch_fn: F,
    ) -> Result<T, E>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
    {
        // Try to get from cache first
        if let Some(cached) = self.get(key).await {
            log::debug!("Cache hit for key: {}", key);
            return Ok(cached);
        }

        // Cache miss - fetch new data
        log::debug!("Cache miss for key: {}", key);
        let data = fetch_fn().await?;
        self.set(key.to_string(), data.clone()).await;
        Ok(data)
    }

    /// Get any cached value ignoring expiration (for fallback scenarios)
    pub async fn get_stale(&self, key: &str) -> Option<T> {
        let cache = self.cache.read().await;
        cache.get(key).map(|entry| entry.data.clone())
    }

    /// Get or fetch with stale fallback: tries to fetch new data, but returns stale cache on error
    pub async fn get_or_fetch_with_stale_fallback<F, Fut, E>(
        &self,
        key: &str,
        fetch_fn: F,
    ) -> Result<T, E>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
    {
        // Try to get fresh cached value first
        if let Some(cached) = self.get(key).await {
            log::debug!("Cache hit for key: {}", key);
            return Ok(cached);
        }

        // Cache miss - try to fetch new data
        log::debug!("Cache miss for key: {}, attempting to fetch", key);
        match fetch_fn().await {
            Ok(data) => {
                // Success - cache and return
                self.set(key.to_string(), data.clone()).await;
                Ok(data)
            }
            Err(e) => {
                // Fetch failed - try to use stale cache as fallback
                if let Some(stale) = self.get_stale(key).await {
                    log::warn!("Fetch failed for key: {}, using stale cache", key);
                    Ok(stale)
                } else {
                    // No stale cache available - propagate error
                    log::error!("Fetch failed for key: {} and no stale cache available", key);
                    Err(e)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_basic() {
        let cache = DataCache::<i32>::new(60);
        
        // Initially empty
        assert!(cache.get("test").await.is_none());
        
        // Set value
        cache.set("test".to_string(), 42).await;
        
        // Get value
        assert_eq!(cache.get("test").await, Some(42));
    }

    #[tokio::test]
    async fn test_cache_expiration() {
        let cache = DataCache::<String>::new(1); // 1 second TTL
        
        cache.set("test".to_string(), "value".to_string()).await;
        assert_eq!(cache.get("test").await, Some("value".to_string()));
        
        // Wait for expiration
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        // Should be expired
        assert!(cache.get("test").await.is_none());
    }

    #[tokio::test]
    async fn test_get_or_fetch() {
        let cache = DataCache::<i32>::new(60);
        let mut call_count = 0;
        
        // First call should fetch
        let result = cache.get_or_fetch("test", || async {
            call_count += 1;
            Ok::<i32, ()>(42)
        }).await;
        assert_eq!(result, Ok(42));
        
        // Second call should use cache (but we can't verify call_count in this simple test)
        let result2 = cache.get_or_fetch("test", || async {
            Ok::<i32, ()>(99)
        }).await;
        assert_eq!(result2, Ok(42)); // Should return cached value
    }

    #[tokio::test]
    async fn test_get_stale() {
        let cache = DataCache::<String>::new(1); // 1 second TTL
        
        cache.set("test".to_string(), "value".to_string()).await;
        
        // Wait for expiration
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        // Normal get should return None (expired)
        assert!(cache.get("test").await.is_none());
        
        // get_stale should still return the value
        assert_eq!(cache.get_stale("test").await, Some("value".to_string()));
    }

    #[tokio::test]
    async fn test_get_or_fetch_with_stale_fallback_success() {
        let cache = DataCache::<i32>::new(60);
        
        // First call should fetch and cache
        let result = cache.get_or_fetch_with_stale_fallback("test", || async {
            Ok::<i32, String>(42)
        }).await;
        assert_eq!(result, Ok(42));
        
        // Second call should return cached value
        let result2 = cache.get_or_fetch_with_stale_fallback("test", || async {
            Ok::<i32, String>(99)
        }).await;
        assert_eq!(result2, Ok(42));
    }

    #[tokio::test]
    async fn test_get_or_fetch_with_stale_fallback_on_error() {
        let cache = DataCache::<i32>::new(1); // 1 second TTL
        
        // First call should fetch and cache
        cache.set("test".to_string(), 42).await;
        
        // Wait for expiration
        tokio::time::sleep(Duration::from_secs(2)).await;
        
        // Try to fetch but fail - should return stale cache
        let result = cache.get_or_fetch_with_stale_fallback("test", || async {
            Err::<i32, String>("API error".to_string())
        }).await;
        assert_eq!(result, Ok(42)); // Should return stale cached value
    }

    #[tokio::test]
    async fn test_get_or_fetch_with_stale_fallback_no_cache() {
        let cache = DataCache::<i32>::new(60);
        
        // Try to fetch but fail with no cache - should propagate error
        let result = cache.get_or_fetch_with_stale_fallback("test", || async {
            Err::<i32, String>("API error".to_string())
        }).await;
        assert_eq!(result, Err("API error".to_string()));
    }
}
