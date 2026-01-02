use crate::types::db_types;
use log::{debug, error, info, warn};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Maximum number of retry attempts for a log entry
const MAX_RETRY_ATTEMPTS: u32 = 10;

/// Entry in the logging queue with retry tracking
#[derive(Debug, Clone)]
pub struct LogQueueEntry {
    pub ac_action: db_types::AcAction,
    pub retry_count: u32,
    pub last_attempt_timestamp: i64,
}

impl LogQueueEntry {
    pub fn new(ac_action: db_types::AcAction) -> Self {
        Self {
            ac_action,
            retry_count: 0,
            last_attempt_timestamp: chrono::Utc::now().timestamp(),
        }
    }

    /// Check if this entry has exceeded max retry attempts
    pub fn has_exceeded_max_retries(&self) -> bool {
        self.retry_count >= MAX_RETRY_ATTEMPTS
    }

    /// Increment retry count and update timestamp
    pub fn record_retry(&mut self) {
        self.retry_count += 1;
        self.last_attempt_timestamp = chrono::Utc::now().timestamp();
    }
}

/// Thread-safe logging queue for AC command log entries
pub struct LoggingQueue {
    entries: Arc<Mutex<Vec<LogQueueEntry>>>,
}

impl LoggingQueue {
    pub fn new() -> Self {
        Self {
            entries: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Add a new entry to the queue
    pub async fn enqueue(&self, ac_action: db_types::AcAction) {
        let mut entries = self.entries.lock().await;
        entries.push(LogQueueEntry::new(ac_action));
        debug!("Enqueued AC action log entry. Queue size: {}", entries.len());
    }

    /// Get the current queue size
    pub async fn size(&self) -> usize {
        let entries = self.entries.lock().await;
        entries.len()
    }

    /// Process the queue, attempting to log all pending entries
    /// Returns (success_count, failure_count, exhausted_count)
    pub async fn process_queue(&self) -> (usize, usize, usize) {
        let mut entries = self.entries.lock().await;
        
        if entries.is_empty() {
            return (0, 0, 0);
        }

        info!("Processing logging queue with {} pending entries", entries.len());
        
        let mut success_count = 0;
        let mut failure_count = 0;
        let mut exhausted_count = 0;
        let mut remaining_entries = Vec::new();

        for mut entry in entries.drain(..) {
            // Check if entry has exceeded max retries
            if entry.has_exceeded_max_retries() {
                error!(
                    "AC action log entry exhausted max retries ({}) - discarding: device={}, action={}",
                    MAX_RETRY_ATTEMPTS,
                    entry.ac_action.device_identifier,
                    entry.ac_action.action_type
                );
                exhausted_count += 1;
                continue;
            }

            // Record the retry attempt
            entry.record_retry();

            // Attempt to insert into database
            match crate::db::ac_actions::insert(entry.ac_action.clone()).await {
                Ok(_) => {
                    debug!(
                        "Successfully logged AC action (retry {}/{}): device={}, action={}",
                        entry.retry_count,
                        MAX_RETRY_ATTEMPTS,
                        entry.ac_action.device_identifier,
                        entry.ac_action.action_type
                    );
                    success_count += 1;
                }
                Err(e) => {
                    warn!(
                        "Failed to log AC action (attempt {}/{}): {} - device={}, action={}",
                        entry.retry_count,
                        MAX_RETRY_ATTEMPTS,
                        e,
                        entry.ac_action.device_identifier,
                        entry.ac_action.action_type
                    );
                    failure_count += 1;
                    // Re-queue for next attempt
                    remaining_entries.push(entry);
                }
            }
        }

        // Update the queue with entries that still need retrying
        *entries = remaining_entries;

        info!(
            "Logging queue processed: {} succeeded, {} failed, {} exhausted, {} remaining",
            success_count, failure_count, exhausted_count, entries.len()
        );

        (success_count, failure_count, exhausted_count)
    }
}

/// Global static instance of the logging queue
static LOGGING_QUEUE: std::sync::OnceLock<LoggingQueue> = std::sync::OnceLock::new();

/// Get or initialize the global logging queue
pub fn get_logging_queue() -> &'static LoggingQueue {
    LOGGING_QUEUE.get_or_init(|| LoggingQueue::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_log_queue_entry_creation() {
        let ac_action = db_types::AcAction::new_for_insert(
            "TestDevice".to_string(),
            "on".to_string(),
            Some(1),
            Some(2),
            Some(25.0),
            Some(0),
            Some(24.5),
            Some(100),
            Some(500),
            None,
            1,
        );

        let entry = LogQueueEntry::new(ac_action.clone());
        assert_eq!(entry.retry_count, 0);
        assert!(!entry.has_exceeded_max_retries());
        assert_eq!(entry.ac_action.device_identifier, "TestDevice");
    }

    #[tokio::test]
    async fn test_retry_count_tracking() {
        let ac_action = db_types::AcAction::new_for_insert(
            "TestDevice".to_string(),
            "off".to_string(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            1,
        );

        let mut entry = LogQueueEntry::new(ac_action);
        
        // Test retry count increments
        for i in 1..=MAX_RETRY_ATTEMPTS {
            entry.record_retry();
            assert_eq!(entry.retry_count, i);
        }
        
        assert!(entry.has_exceeded_max_retries());
    }

    #[tokio::test]
    async fn test_queue_enqueue_and_size() {
        let queue = LoggingQueue::new();
        assert_eq!(queue.size().await, 0);

        let ac_action = db_types::AcAction::new_for_insert(
            "TestDevice".to_string(),
            "on".to_string(),
            Some(1),
            Some(2),
            Some(25.0),
            Some(0),
            None,
            None,
            None,
            None,
            1,
        );

        queue.enqueue(ac_action.clone()).await;
        assert_eq!(queue.size().await, 1);

        queue.enqueue(ac_action).await;
        assert_eq!(queue.size().await, 2);
    }
}
