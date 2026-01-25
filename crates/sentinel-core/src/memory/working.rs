//! Working Memory - Hot cache with LRU eviction
//!
//! Fast access to recently used memories (10 items max).

use super::{MemoryItem, MemoryQueryResult, MemorySource};
use std::collections::{HashMap, VecDeque};
use uuid::Uuid;

/// Working memory with LRU eviction
#[derive(Debug)]
pub struct WorkingMemory {
    /// Maximum capacity
    capacity: usize,

    /// Storage for memory items
    items: HashMap<Uuid, MemoryItem>,

    /// LRU queue (most recent at back)
    lru_queue: VecDeque<Uuid>,
}

impl WorkingMemory {
    /// Create a new working memory with default capacity (10)
    pub fn new() -> Self {
        Self::with_capacity(10)
    }

    /// Create a working memory with custom capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            capacity,
            items: HashMap::new(),
            lru_queue: VecDeque::new(),
        }
    }

    /// Store a memory item (may evict LRU item if full)
    pub fn store(&mut self, mut item: MemoryItem) -> Option<MemoryItem> {
        let id = item.id;

        // If already exists, update and move to back
        if self.items.contains_key(&id) {
            self.lru_queue.retain(|&x| x != id);
            self.lru_queue.push_back(id);
            item.access();
            self.items.insert(id, item);
            return None;
        }

        // Evict LRU if at capacity
        let evicted = if self.items.len() >= self.capacity {
            self.lru_queue
                .pop_front()
                .and_then(|evicted_id| self.items.remove(&evicted_id))
        } else {
            None
        };

        // Insert new item
        self.items.insert(id, item);
        self.lru_queue.push_back(id);

        evicted
    }

    /// Get a memory item by ID (marks as accessed)
    pub fn get(&mut self, id: &Uuid) -> Option<&mut MemoryItem> {
        if self.items.contains_key(id) {
            // Move to back of queue
            self.lru_queue.retain(|&x| x != *id);
            self.lru_queue.push_back(*id);

            // Mark as accessed
            if let Some(item) = self.items.get_mut(id) {
                item.access();
                return Some(item);
            }
        }
        None
    }

    /// Query memories by content similarity (simple substring match)
    pub fn query(&mut self, query: &str, limit: usize) -> Vec<MemoryQueryResult> {
        let query_lower = query.to_lowercase();
        let mut results: Vec<_> = self
            .items
            .values_mut()
            .filter(|item| {
                item.content.to_lowercase().contains(&query_lower)
                    || item
                        .tags
                        .iter()
                        .any(|tag| tag.to_lowercase().contains(&query_lower))
            })
            .map(|item| {
                item.access();

                // Simple relevance scoring based on substring match quality
                let content_lower = item.content.to_lowercase();
                let match_score = if content_lower == query_lower {
                    1.0
                } else if content_lower.starts_with(&query_lower) {
                    0.9
                } else {
                    0.7
                };

                MemoryQueryResult {
                    item: item.clone(),
                    score: match_score * item.relevance_score(),
                    source: MemorySource::Working,
                }
            })
            .collect();

        // Sort by score descending
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.truncate(limit);

        results
    }

    /// Get all items sorted by recency (most recent first)
    pub fn get_all(&self) -> Vec<&MemoryItem> {
        self.lru_queue
            .iter()
            .rev()
            .filter_map(|id| self.items.get(id))
            .collect()
    }

    /// Check if memory contains an item
    pub fn contains(&self, id: &Uuid) -> bool {
        self.items.contains_key(id)
    }

    /// Get current size
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Get capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Clear all memories
    pub fn clear(&mut self) {
        self.items.clear();
        self.lru_queue.clear();
    }

    /// Get the least recently used item (without removing)
    pub fn peek_lru(&self) -> Option<&MemoryItem> {
        self.lru_queue.front().and_then(|id| self.items.get(id))
    }

    /// Get the most recently used item
    pub fn peek_mru(&self) -> Option<&MemoryItem> {
        self.lru_queue.back().and_then(|id| self.items.get(id))
    }
}

impl Default for WorkingMemory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::MemoryType;

    #[test]
    fn test_working_memory_creation() {
        let wm = WorkingMemory::new();
        assert_eq!(wm.capacity(), 10);
        assert_eq!(wm.len(), 0);
        assert!(wm.is_empty());
    }

    #[test]
    fn test_working_memory_store() {
        let mut wm = WorkingMemory::new();
        let item = MemoryItem::new("Test memory".to_string(), MemoryType::Action);
        let id = item.id;

        let evicted = wm.store(item);
        assert!(evicted.is_none());
        assert_eq!(wm.len(), 1);
        assert!(wm.contains(&id));
    }

    #[test]
    fn test_lru_eviction() {
        let mut wm = WorkingMemory::with_capacity(3);

        let item1 = MemoryItem::new("Memory 1".to_string(), MemoryType::Action);
        let item2 = MemoryItem::new("Memory 2".to_string(), MemoryType::Action);
        let item3 = MemoryItem::new("Memory 3".to_string(), MemoryType::Action);
        let item4 = MemoryItem::new("Memory 4".to_string(), MemoryType::Action);

        let id1 = item1.id;

        wm.store(item1);
        wm.store(item2);
        wm.store(item3);

        // Should evict item1
        let evicted = wm.store(item4);
        assert!(evicted.is_some());
        assert_eq!(evicted.unwrap().id, id1);
        assert_eq!(wm.len(), 3);
        assert!(!wm.contains(&id1));
    }

    #[test]
    fn test_get_updates_lru() {
        let mut wm = WorkingMemory::with_capacity(3);

        let item1 = MemoryItem::new("Memory 1".to_string(), MemoryType::Action);
        let item2 = MemoryItem::new("Memory 2".to_string(), MemoryType::Action);
        let item3 = MemoryItem::new("Memory 3".to_string(), MemoryType::Action);

        let id1 = item1.id;
        let id2 = item2.id;

        wm.store(item1);
        wm.store(item2);
        wm.store(item3);

        // Access item1, making it most recent
        wm.get(&id1);

        // Now item2 should be LRU
        let lru = wm.peek_lru().unwrap();
        assert_eq!(lru.id, id2);
    }

    #[test]
    fn test_query() {
        let mut wm = WorkingMemory::new();

        let item1 = MemoryItem::builder()
            .content("Implemented authentication".to_string())
            .memory_type(MemoryType::Action)
            .tag("auth".to_string())
            .build()
            .unwrap();

        let item2 = MemoryItem::builder()
            .content("Fixed database bug".to_string())
            .memory_type(MemoryType::Action)
            .build()
            .unwrap();

        wm.store(item1);
        wm.store(item2);

        let results = wm.query("authentication", 10);
        assert_eq!(results.len(), 1);
        assert!(results[0].item.content.contains("authentication"));
    }

    #[test]
    fn test_peek_lru_mru() {
        let mut wm = WorkingMemory::new();

        let item1 = MemoryItem::new("First".to_string(), MemoryType::Action);
        let item2 = MemoryItem::new("Second".to_string(), MemoryType::Action);

        let id1 = item1.id;
        let id2 = item2.id;

        wm.store(item1);
        wm.store(item2);

        assert_eq!(wm.peek_lru().unwrap().id, id1);
        assert_eq!(wm.peek_mru().unwrap().id, id2);
    }

    #[test]
    fn test_clear() {
        let mut wm = WorkingMemory::new();

        wm.store(MemoryItem::new("Test".to_string(), MemoryType::Action));
        assert_eq!(wm.len(), 1);

        wm.clear();
        assert_eq!(wm.len(), 0);
        assert!(wm.is_empty());
    }

    #[test]
    fn test_get_all_ordered() {
        let mut wm = WorkingMemory::new();

        let item1 = MemoryItem::new("First".to_string(), MemoryType::Action);
        let item2 = MemoryItem::new("Second".to_string(), MemoryType::Action);
        let item3 = MemoryItem::new("Third".to_string(), MemoryType::Action);

        wm.store(item1);
        wm.store(item2);
        wm.store(item3);

        let all = wm.get_all();
        assert_eq!(all.len(), 3);
        // Most recent first
        assert_eq!(all[0].content, "Third");
        assert_eq!(all[2].content, "First");
    }
}
