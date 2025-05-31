use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::path::Path;
use anyhow::Result;

/// Generic trait for cacheable content
pub trait CacheableContent: Clone + Send + Sync {
    /// Get the total number of pages/sheets/slides
    fn total_units(&self) -> Option<usize>;
    
    /// Get the full content as string
    fn full_content(&self) -> &str;
    
    /// Get character indices for efficient slicing
    fn char_indices(&self) -> &[usize];
    
    /// Calculate memory usage estimate
    fn memory_usage(&self) -> usize {
        self.full_content().len() + self.char_indices().len() * std::mem::size_of::<usize>()
    }
}

/// Generic cache entry
#[derive(Debug, Clone)]
pub struct CacheEntry<T: CacheableContent> {
    pub content: T,
    pub file_path: String,
    pub last_modified: Option<std::time::SystemTime>,
}

impl<T: CacheableContent> CacheEntry<T> {
    pub fn new(content: T, file_path: String) -> Self {
        let last_modified = std::fs::metadata(&file_path)
            .and_then(|metadata| metadata.modified())
            .ok();
        
        Self {
            content,
            file_path,
            last_modified,
        }
    }
    
    /// Check if the cache entry is still valid (file hasn't been modified)
    pub fn is_valid(&self) -> bool {
        if let Some(cached_time) = self.last_modified {
            if let Ok(metadata) = std::fs::metadata(&self.file_path) {
                if let Ok(current_time) = metadata.modified() {
                    return current_time <= cached_time;
                }
            }
        }
        // If we can't determine modification time, assume it's still valid
        true
    }
}

/// Macro to create a caching system for a specific file type
#[macro_export]
macro_rules! create_file_cache {
    (
        $cache_name:ident,
        $cache_type:ty,
        $extractor_fn:expr,
        $page_count_fn:expr
    ) => {
        use std::collections::HashMap;
        use std::sync::{Arc, Mutex};
        use $crate::cache_system::{CacheEntry, CacheableContent};
        
        type $cache_name = Arc<Mutex<HashMap<String, CacheEntry<$cache_type>>>>;
        
        lazy_static::lazy_static! {
            static ref CACHE: $cache_name = Arc::new(Mutex::new(HashMap::new()));
        }
        
        /// Get or create cached content
        pub fn get_or_cache_content(file_path: &str) -> Result<$cache_type> {
            let cache_key = file_path.to_string();
            
            // Check if already cached and valid
            {
                let cache = CACHE.lock().unwrap();
                if let Some(cached_entry) = cache.get(&cache_key) {
                    if cached_entry.is_valid() {
                        return Ok(cached_entry.content.clone());
                    }
                }
            }
            
            // Extract content using the provided function
            let content = $extractor_fn(file_path)?;
            
            // Store in cache
            {
                let mut cache = CACHE.lock().unwrap();
                let entry = CacheEntry::new(content.clone(), cache_key.clone());
                cache.insert(cache_key, entry);
            }
            
            Ok(content)
        }
        
        /// Clear the cache
        pub fn clear_cache() {
            let mut cache = CACHE.lock().unwrap();
            cache.clear();
        }
        
        /// Get cache statistics
        pub fn get_cache_stats() -> (usize, usize) {
            let cache = CACHE.lock().unwrap();
            let num_files = cache.len();
            let total_memory = cache.values()
                .map(|entry| entry.content.memory_usage())
                .sum();
            (num_files, total_memory)
        }
        
        /// Remove invalid cache entries
        pub fn cleanup_cache() {
            let mut cache = CACHE.lock().unwrap();
            cache.retain(|_, entry| entry.is_valid());
        }
        
        /// Extract specific pages/units from cached content
        pub fn extract_units_from_cache(
            cached_content: &$cache_type,
            unit_numbers: &[usize],
            file_path: &str,
        ) -> Result<String> {
            if let Some(_total_units) = cached_content.total_units() {
                // Use page-specific extraction if available
                $page_count_fn(file_path, unit_numbers)
            } else {
                // Fallback to returning full content with a note
                let mut result = String::new();
                result.push_str(&format!("# {}\n\n", 
                    Path::new(file_path).file_name().unwrap().to_string_lossy()));
                result.push_str(&format!("## Content (Requested Units: {:?})\n\n", unit_numbers));
                result.push_str("*Note: Unit-specific extraction not available. Returning full document.*\n\n");
                result.push_str(cached_content.full_content());
                Ok(result)
            }
        }
        
        /// Extract a character range from cached content
        pub fn extract_char_range_from_cache(
            cached_content: &$cache_type,
            start_char: usize,
            end_char: usize,
        ) -> Result<String> {
            let char_indices = cached_content.char_indices();
            let total_chars = char_indices.len().saturating_sub(1);
            
            if start_char >= total_chars {
                return Ok(String::new());
            }
            
            let actual_end = std::cmp::min(end_char, total_chars);
            
            // Extract the chunk using pre-computed byte indices
            let start_byte = char_indices[start_char];
            let end_byte = if actual_end < char_indices.len() {
                char_indices[actual_end]
            } else {
                cached_content.full_content().len()
            };
            
            Ok(cached_content.full_content()[start_byte..end_byte].to_string())
        }
    };
}

/// Macro to implement CacheableContent for a struct
#[macro_export]
macro_rules! impl_cacheable_content {
    ($struct_name:ty, $content_field:ident, $char_indices_field:ident, $total_units_field:ident) => {
        impl CacheableContent for $struct_name {
            fn total_units(&self) -> Option<usize> {
                self.$total_units_field
            }
            
            fn full_content(&self) -> &str {
                &self.$content_field
            }
            
            fn char_indices(&self) -> &[usize] {
                &self.$char_indices_field
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct TestCache {
        content: String,
        char_indices: Vec<usize>,
        total_pages: Option<usize>,
    }

    impl_cacheable_content!(TestCache, content, char_indices, total_pages);

    #[test]
    fn test_cacheable_content_trait() {
        let test_cache = TestCache {
            content: "Hello, world!".to_string(),
            char_indices: vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13],
            total_pages: Some(1),
        };

        assert_eq!(test_cache.total_units(), Some(1));
        assert_eq!(test_cache.full_content(), "Hello, world!");
        assert_eq!(test_cache.char_indices().len(), 14);
        assert!(test_cache.memory_usage() > 0);
    }

    #[test]
    fn test_cache_entry() {
        let test_cache = TestCache {
            content: "Test content".to_string(),
            char_indices: vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
            total_pages: Some(1),
        };

        let entry = CacheEntry::new(test_cache, "test.txt".to_string());
        assert_eq!(entry.file_path, "test.txt");
        // Note: is_valid() will return true for non-existent files in this implementation
    }
} 