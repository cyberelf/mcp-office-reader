use std::path::{Path, PathBuf};
use std::env;
use anyhow::{Result, Context};
use crate::fast_pdf_extractor::FastPdfExtractor;
use crate::cache_system::CacheManager;
use crate::impl_cacheable_content;

/// Cache for storing extracted PDF content to avoid re-parsing
#[derive(Debug, Clone)]
pub struct PdfCache {
    pub content: String,
    pub char_indices: Vec<usize>, // Byte indices for each character for efficient slicing
    pub total_pages: Option<usize>,
}

// Implement CacheableContent for PdfCache
impl_cacheable_content!(PdfCache, content, char_indices, total_pages);
 
lazy_static::lazy_static! {
    /// Global PDF cache manager
    static ref PDF_CACHE_MANAGER: CacheManager<PdfCache> = CacheManager::new();
}

/// Function to extract PDF content and create cache
fn extract_pdf_content(file_path: &str) -> Result<PdfCache> {
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
    
    Ok(PdfCache {
        content: full_text,
        char_indices,
        total_pages,
    })
}

/// Function to extract specific pages from PDF
fn extract_pdf_pages(file_path: &str, page_numbers: &[usize]) -> Result<String> {
    FastPdfExtractor::extract_pages_text(file_path, page_numbers)
        .with_context(|| format!("Failed to extract specific pages from PDF: {}", file_path))
}

/// Get or create cached PDF content with page count information
pub fn get_or_cache_pdf_content(file_path: &str) -> Result<PdfCache> {
    PDF_CACHE_MANAGER.get_or_cache(file_path, extract_pdf_content)
}

/// Extract specific pages from a cached PDF
pub fn extract_pages_from_cache(
    pdf_cache: &PdfCache,
    page_numbers: &[usize],
    file_path: &str,
) -> Result<String> {
    PDF_CACHE_MANAGER.extract_units(pdf_cache, page_numbers, file_path, extract_pdf_pages)
}

/// Extract a character range from cached PDF content
pub fn extract_char_range_from_cache(
    pdf_cache: &PdfCache,
    start_char: usize,
    end_char: usize,
) -> Result<String> {
    PDF_CACHE_MANAGER.extract_char_range(pdf_cache, start_char, end_char)
}

/// Clear the PDF cache
pub fn clear_pdf_cache() {
    PDF_CACHE_MANAGER.clear();
}

/// Clear the Excel cache
pub fn clear_excel_cache() {
    use crate::document_parser::EXCEL_CACHE_MANAGER;
    EXCEL_CACHE_MANAGER.clear();
}

/// Clear the DOCX cache
pub fn clear_docx_cache() {
    use crate::document_parser::DOCX_CACHE_MANAGER;
    DOCX_CACHE_MANAGER.clear();
}

/// Clear the PowerPoint cache
pub fn clear_powerpoint_cache() {
    use crate::powerpoint_parser::POWERPOINT_CACHE_MANAGER;
    POWERPOINT_CACHE_MANAGER.clear();
}

/// Clear all document caches
pub fn clear_all_caches() {
    clear_pdf_cache();
    clear_excel_cache();
    clear_docx_cache();
    clear_powerpoint_cache();
}

/// Get cache statistics (number of cached files, total memory usage estimate)
pub fn get_cache_stats() -> (usize, usize) {
    PDF_CACHE_MANAGER.get_stats()
}

/// Get comprehensive cache statistics for all document types
pub fn get_all_cache_stats() -> (usize, usize) {
    use crate::document_parser::{EXCEL_CACHE_MANAGER, DOCX_CACHE_MANAGER};
    use crate::powerpoint_parser::POWERPOINT_CACHE_MANAGER;
    
    let (pdf_files, pdf_memory) = PDF_CACHE_MANAGER.get_stats();
    let (excel_files, excel_memory) = EXCEL_CACHE_MANAGER.get_stats();
    let (docx_files, docx_memory) = DOCX_CACHE_MANAGER.get_stats();
    let (ppt_files, ppt_memory) = POWERPOINT_CACHE_MANAGER.get_stats();
    
    let total_files = pdf_files + excel_files + docx_files + ppt_files;
    let total_memory = pdf_memory + excel_memory + docx_memory + ppt_memory;
    
    (total_files, total_memory)
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

/// Resolve a file path with security checks
/// When PROJECT_ROOT is configured, absolute paths are rejected for security
pub fn resolve_file_path(file_path: &str) -> Result<PathBuf, String> {
    let path = Path::new(file_path);
    
    // Check if PROJECT_ROOT is configured
    let project_root_configured = env::var("PROJECT_ROOT").is_ok();
    
    // Security check: reject absolute paths when PROJECT_ROOT is configured
    if path.is_absolute() && project_root_configured {
        return Err("Absolute paths are not allowed when PROJECT_ROOT is configured for security reasons".to_string());
    }
    
    // If it's an absolute path and PROJECT_ROOT is not configured, allow it
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    
    // For relative paths, try to resolve using the PROJECT_ROOT environment variable
    if let Ok(project_root) = env::var("PROJECT_ROOT") {
        let project_root_path = Path::new(&project_root);
        if project_root_path.exists() {
            return Ok(project_root_path.join(path));
        } else {
            return Err(format!("PROJECT_ROOT directory does not exist: {}", project_root));
        }
    }
    
    // If PROJECT_ROOT is not set, use current directory
    match env::current_dir() {
        Ok(current_dir) => Ok(current_dir.join(path)),
        Err(e) => Err(format!("Failed to get current directory: {}", e)),
    }
}

/// Check if a file exists and determine its type (expects already resolved path)
pub fn validate_file_path(resolved_path: &str) -> Result<String, String> {
    let path = Path::new(resolved_path);
    
    if !path.exists() {
        return Err(format!("File not found: {}", resolved_path));
    }
    
    // Determine file type from extension
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase());
    
    match extension {
        Some(ext) => {
            match ext.as_str() {
                "pdf" | "xlsx" | "xls" | "docx" | "doc" | "pptx" | "ppt" => Ok(ext),
                _ => Err(format!("Unsupported file type: .{}", ext)),
            }
        },
        None => Err("Unable to determine file type from extension".to_string()),
    }
}

/// Resolve a file path and return it as a string
pub fn resolve_file_path_string(file_path: &str) -> Result<String, String> {
    resolve_file_path(file_path).map(|path| path.to_string_lossy().to_string())
}

/// Generate a markdown header for a file (expects already resolved path)
pub fn generate_file_header(resolved_path: &str) -> String {
    let path = Path::new(resolved_path);
    format!("# {}\n\n", path.file_name().unwrap().to_string_lossy())
}

/// Generate a markdown header for a chunk/page
pub fn generate_chunk_header(chunk_num: usize, start_pos: usize, end_pos: usize, unit: &str) -> String {
    format!("## {} {} (chars {}-{})\n\n", unit, chunk_num, start_pos, end_pos)
}

/// Break text at word boundary to avoid cutting words in half
pub fn break_at_word_boundary(text: &str, max_chars: usize) -> &str {
    if text.len() <= max_chars {
        return text;
    }
    
    // Find the last space before max_chars
    let mut break_point = max_chars;
    let chars: Vec<char> = text.chars().collect();
    
    while break_point > 0 && chars[break_point - 1] != ' ' {
        break_point -= 1;
    }
    
    // If no space found, just cut at max_chars
    if break_point == 0 {
        break_point = max_chars;
    }
    
    // Convert back to byte index
    let byte_index = chars.iter().take(break_point).map(|c| c.len_utf8()).sum();
    &text[..byte_index]
} 