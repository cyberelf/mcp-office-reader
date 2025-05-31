use std::path::Path;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use anyhow::{Result, Context};
use crate::fast_pdf_extractor::FastPdfExtractor;

/// Cache for storing extracted PDF content to avoid re-parsing
#[derive(Debug, Clone)]
pub struct PdfCache {
    pub content: String,
    pub char_indices: Vec<usize>, // Byte indices for each character for efficient slicing
    pub total_pages: Option<usize>,
}

/// Global cache for PDF content (thread-safe)
type GlobalPdfCache = Arc<Mutex<HashMap<String, PdfCache>>>;

lazy_static::lazy_static! {
    static ref PDF_CACHE: GlobalPdfCache = Arc::new(Mutex::new(HashMap::new()));
}

/// Parse a comma-separated string of page numbers and ranges
/// Examples: "1,3,5-7" -> [1,3,5,6,7], "all" -> None (meaning all pages)
pub fn parse_pages_parameter(pages: &str, total_pages: usize) -> Result<Vec<usize>, String> {
    if pages.trim().is_empty() || pages.trim().to_lowercase() == "all" {
        return Ok((1..=total_pages).collect());
    }
    
    let mut page_numbers = Vec::new();
    
    for part in pages.split(',') {
        let part = part.trim();
        
        if part.contains('-') {
            // Handle range like "5-7"
            let range_parts: Vec<&str> = part.split('-').collect();
            if range_parts.len() != 2 {
                return Err(format!("Invalid range format: {}", part));
            }
            
            let start: usize = range_parts[0].trim().parse()
                .map_err(|_| format!("Invalid page number: {}", range_parts[0]))?;
            let end: usize = range_parts[1].trim().parse()
                .map_err(|_| format!("Invalid page number: {}", range_parts[1]))?;
            
            if start == 0 || end == 0 {
                return Err("Page numbers must start from 1".to_string());
            }
            
            if start > end {
                return Err(format!("Invalid range: {} > {}", start, end));
            }
            
            if end > total_pages {
                return Err(format!("Page {} exceeds total pages ({})", end, total_pages));
            }
            
            for page in start..=end {
                if !page_numbers.contains(&page) {
                    page_numbers.push(page);
                }
            }
        } else {
            // Handle single page number
            let page: usize = part.parse()
                .map_err(|_| format!("Invalid page number: {}", part))?;
            
            if page == 0 {
                return Err("Page numbers must start from 1".to_string());
            }
            
            if page > total_pages {
                return Err(format!("Page {} exceeds total pages ({})", page, total_pages));
            }
            
            if !page_numbers.contains(&page) {
                page_numbers.push(page);
            }
        }
    }
    
    page_numbers.sort();
    Ok(page_numbers)
}

/// Get or create cached PDF content with page count information
pub fn get_or_cache_pdf_content(file_path: &str) -> Result<PdfCache> {
    let cache_key = file_path.to_string();
    
    // Check if already cached
    {
        let cache = PDF_CACHE.lock().unwrap();
        if let Some(cached) = cache.get(&cache_key) {
            return Ok(cached.clone());
        }
    }
    
    // Extract PDF content and get page count (only once per file)
    let full_text = FastPdfExtractor::extract_text(file_path)
        .with_context(|| format!("Failed to extract text from PDF: {}", file_path))?;
    
    let total_pages = FastPdfExtractor::get_page_count(file_path)
        .with_context(|| format!("Failed to get page count from PDF: {}", file_path))
        .ok(); // Make it optional in case page counting fails
    
    // Pre-compute character byte indices for efficient slicing
    let mut char_indices = Vec::new();
    let mut byte_pos = 0;
    
    for ch in full_text.chars() {
        char_indices.push(byte_pos);
        byte_pos += ch.len_utf8();
    }
    char_indices.push(byte_pos); // Add final position
    
    let pdf_cache = PdfCache {
        content: full_text,
        char_indices,
        total_pages,
    };
    
    // Store in cache
    {
        let mut cache = PDF_CACHE.lock().unwrap();
        cache.insert(cache_key, pdf_cache.clone());
    }
    
    Ok(pdf_cache)
}

/// Extract specific pages from a cached PDF
pub fn extract_pages_from_cache(
    pdf_cache: &PdfCache,
    page_numbers: &[usize],
    file_path: &str,
) -> Result<String> {
    if let Some(_total_pages) = pdf_cache.total_pages {
        // Use the new page-specific extraction if we have page count
        FastPdfExtractor::extract_pages_text(file_path, page_numbers)
            .with_context(|| format!("Failed to extract specific pages from PDF: {}", file_path))
    } else {
        // Fallback to returning full content with a note
        let mut result = String::new();
        result.push_str(&format!("# {}\n\n", Path::new(file_path).file_name().unwrap().to_string_lossy()));
        result.push_str(&format!("## Content (Requested Pages: {:?})\n\n", page_numbers));
        result.push_str("*Note: Page-specific extraction not available. Returning full document.*\n\n");
        result.push_str(&pdf_cache.content);
        Ok(result)
    }
}

/// Extract a character range from cached PDF content
pub fn extract_char_range_from_cache(
    pdf_cache: &PdfCache,
    start_char: usize,
    end_char: usize,
) -> Result<String> {
    let total_chars = pdf_cache.char_indices.len().saturating_sub(1);
    
    if start_char >= total_chars {
        return Ok(String::new());
    }
    
    let actual_end = std::cmp::min(end_char, total_chars);
    
    // Extract the chunk using pre-computed byte indices
    let start_byte = pdf_cache.char_indices[start_char];
    let end_byte = if actual_end < pdf_cache.char_indices.len() {
        pdf_cache.char_indices[actual_end]
    } else {
        pdf_cache.content.len()
    };
    
    Ok(pdf_cache.content[start_byte..end_byte].to_string())
}

/// Clear the PDF cache
pub fn clear_pdf_cache() {
    let mut cache = PDF_CACHE.lock().unwrap();
    cache.clear();
}

/// Get cache statistics (number of cached files, total memory usage estimate)
pub fn get_cache_stats() -> (usize, usize) {
    let cache = PDF_CACHE.lock().unwrap();
    let num_files = cache.len();
    let total_memory = cache.values()
        .map(|pdf_cache| pdf_cache.content.len() + pdf_cache.char_indices.len() * std::mem::size_of::<usize>())
        .sum();
    (num_files, total_memory)
}

/// Check if a file exists and determine its type
pub fn validate_file_path(file_path: &str) -> Result<String, String> {
    if !Path::new(file_path).exists() {
        return Err(format!("File not found: {}", file_path));
    }
    
    // Determine file type from extension
    let extension = Path::new(file_path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase());
    
    match extension {
        Some(ext) => {
            match ext.as_str() {
                "pdf" | "xlsx" | "xls" | "docx" | "doc" | "pptx" | "ppt" => Ok(ext),
                _ => Err(format!("Unsupported file type: {}", ext)),
            }
        }
        None => Err("Unable to determine file type (no extension)".to_string()),
    }
}

/// Generate markdown header for a file
pub fn generate_file_header(file_path: &str) -> String {
    format!("# {}\n\n", Path::new(file_path).file_name().unwrap().to_string_lossy())
}

/// Generate chunk header for streaming
pub fn generate_chunk_header(chunk_num: usize, start_pos: usize, end_pos: usize, unit: &str) -> String {
    format!("## Chunk {} ({} {}-{})\n\n", chunk_num, unit, start_pos, end_pos)
}

/// Break text at word boundaries for better readability
pub fn break_at_word_boundary(text: &str, max_chars: usize) -> &str {
    if text.chars().count() <= max_chars {
        return text;
    }
    
    // First, truncate to max_chars
    let mut truncated_end = 0;
    let mut char_count = 0;
    for (byte_idx, _) in text.char_indices() {
        if char_count >= max_chars {
            truncated_end = byte_idx;
            break;
        }
        char_count += 1;
    }
    
    if truncated_end == 0 {
        return text; // Fallback if something goes wrong
    }
    
    let truncated_text = &text[..truncated_end];
    
    // Now try to find a word boundary within the truncated text
    if let Some(last_space_pos) = truncated_text.rfind(' ') {
        let word_boundary_chunk = &truncated_text[..last_space_pos];
        // Ensure we make meaningful progress (at least 10% of max_chars or minimum 10 chars)
        let min_progress = std::cmp::max(max_chars / 10, 10);
        if word_boundary_chunk.chars().count() >= min_progress {
            return word_boundary_chunk;
        }
    }
    
    // If word boundary breaking doesn't work well, return the truncated text
    truncated_text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pages_parameter() {
        // Test "all" parameter
        assert_eq!(parse_pages_parameter("all", 5).unwrap(), vec![1, 2, 3, 4, 5]);
        assert_eq!(parse_pages_parameter("", 3).unwrap(), vec![1, 2, 3]);
        
        // Test single pages
        assert_eq!(parse_pages_parameter("1", 5).unwrap(), vec![1]);
        assert_eq!(parse_pages_parameter("3", 5).unwrap(), vec![3]);
        
        // Test multiple pages
        assert_eq!(parse_pages_parameter("1,3,5", 5).unwrap(), vec![1, 3, 5]);
        assert_eq!(parse_pages_parameter("5,1,3", 5).unwrap(), vec![1, 3, 5]); // Should be sorted
        
        // Test ranges
        assert_eq!(parse_pages_parameter("1-3", 5).unwrap(), vec![1, 2, 3]);
        assert_eq!(parse_pages_parameter("2-4", 5).unwrap(), vec![2, 3, 4]);
        
        // Test mixed ranges and single pages
        assert_eq!(parse_pages_parameter("1,3-5", 5).unwrap(), vec![1, 3, 4, 5]);
        assert_eq!(parse_pages_parameter("1-2,4,6-7", 10).unwrap(), vec![1, 2, 4, 6, 7]);
        
        // Test duplicates (should be removed)
        assert_eq!(parse_pages_parameter("1,1,2", 5).unwrap(), vec![1, 2]);
        assert_eq!(parse_pages_parameter("1-3,2-4", 5).unwrap(), vec![1, 2, 3, 4]);
        
        // Test error cases
        assert!(parse_pages_parameter("0", 5).is_err()); // Page 0 not allowed
        assert!(parse_pages_parameter("6", 5).is_err()); // Page exceeds total
        assert!(parse_pages_parameter("3-2", 5).is_err()); // Invalid range
        assert!(parse_pages_parameter("1-6", 5).is_err()); // Range exceeds total
        assert!(parse_pages_parameter("abc", 5).is_err()); // Invalid number
        assert!(parse_pages_parameter("1-2-3", 5).is_err()); // Invalid range format
    }

    #[test]
    fn test_validate_file_path() {
        // Test non-existent file
        assert!(validate_file_path("nonexistent.pdf").is_err());
        
        // Test unsupported extension
        assert!(validate_file_path("test.txt").is_err());
        
        // Test no extension
        assert!(validate_file_path("test").is_err());
    }

    #[test]
    fn test_generate_file_header() {
        let header = generate_file_header("/path/to/document.pdf");
        assert_eq!(header, "# document.pdf\n\n");
    }

    #[test]
    fn test_generate_chunk_header() {
        let header = generate_chunk_header(1, 0, 1000, "characters");
        assert_eq!(header, "## Chunk 1 (characters 0-1000)\n\n");
    }

    #[test]
    fn test_break_at_word_boundary() {
        let text = "This is a long sentence that should be broken at word boundaries.";
        let result = break_at_word_boundary(text, 30);
        assert!(result.chars().count() <= 30);
        assert!(result.ends_with("that")); // Should break at word boundary before "should"
        
        // Test with text shorter than max
        let short_text = "Short text";
        let result = break_at_word_boundary(short_text, 30);
        assert_eq!(result, short_text);
    }

    #[test]
    fn test_cache_management() {
        // Clear cache first
        clear_pdf_cache();
        
        let (num_files, _) = get_cache_stats();
        assert_eq!(num_files, 0);
        
        // Cache stats should work
        let (files, memory) = get_cache_stats();
        assert_eq!(files, 0);
        assert_eq!(memory, 0);
    }

    #[test]
    fn test_extract_char_range_from_cache() {
        let content = "Hello, world! This is a test.".to_string();
        let mut char_indices = Vec::new();
        let mut byte_pos = 0;
        
        for ch in content.chars() {
            char_indices.push(byte_pos);
            byte_pos += ch.len_utf8();
        }
        char_indices.push(byte_pos);
        
        let cache = PdfCache {
            content,
            char_indices,
            total_pages: Some(1),
        };
        
        let result = extract_char_range_from_cache(&cache, 0, 5).unwrap();
        assert_eq!(result, "Hello");
        
        let result = extract_char_range_from_cache(&cache, 7, 12).unwrap();
        assert_eq!(result, "world");
    }
} 