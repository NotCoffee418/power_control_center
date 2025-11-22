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
}
