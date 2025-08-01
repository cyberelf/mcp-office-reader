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

/// Generic cache manager
pub struct CacheManager<T: CacheableContent> {
    cache: Arc<Mutex<HashMap<String, CacheEntry<T>>>>,
}

impl<T: CacheableContent> CacheManager<T> {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    /// Get or create cached content
    pub fn get_or_cache<F>(&self, file_path: &str, extractor: F) -> Result<T>
    where
        F: FnOnce(&str) -> Result<T>,
    {
        let cache_key = file_path.to_string();
        
        // Check if already cached and valid
        {
            let cache = self.cache.lock().unwrap();
            if let Some(cached_entry) = cache.get(&cache_key) {
                if cached_entry.is_valid() {
                    return Ok(cached_entry.content.clone());
                }
            }
        }
        
        // Extract content using the provided function
        let content = extractor(file_path)?;
        
        // Store in cache
        {
            let mut cache = self.cache.lock().unwrap();
            let entry = CacheEntry::new(content.clone(), cache_key.clone());
            cache.insert(cache_key, entry);
        }
        
        Ok(content)
    }
    
    /// Clear the cache
    pub fn clear(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.clear();
    }
    
    /// Get cache statistics
    pub fn get_stats(&self) -> (usize, usize) {
        let cache = self.cache.lock().unwrap();
        let num_files = cache.len();
        let total_memory = cache.values()
            .map(|entry| entry.content.memory_usage())
            .sum();
        (num_files, total_memory)
    }
    
    /// Remove invalid cache entries
    pub fn cleanup(&self) {
        let mut cache = self.cache.lock().unwrap();
        cache.retain(|_, entry| entry.is_valid());
    }
    
    /// Extract specific pages/units from cached content
    pub fn extract_units<F>(
        &self,
        cached_content: &T,
        unit_numbers: &[usize],
        file_path: &str,
        extractor: F,
    ) -> Result<String>
    where
        F: FnOnce(&str, &[usize]) -> Result<String>,
    {
        if let Some(_total_units) = cached_content.total_units() {
            // Use page-specific extraction if available
            extractor(file_path, unit_numbers)
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
    pub fn extract_char_range(
        &self,
        cached_content: &T,
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
}

impl<T: CacheableContent> Default for CacheManager<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Macro to implement CacheableContent for a struct
#[macro_export]
macro_rules! impl_cacheable_content {
    ($struct_name:ty, $content_field:ident, $char_indices_field:ident, $total_units_field:ident) => {
        impl $crate::cache_system::CacheableContent for $struct_name {
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