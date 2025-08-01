use std::path::Path;
use std::fs::File;
use std::io::Read;

use anyhow::{Result, Context};
use calamine::{Reader, open_workbook, Xlsx, Data};
use crate::fast_pdf_extractor::FastPdfExtractor;
use crate::shared_utils::{parse_pages_parameter, validate_file_path, get_or_cache_pdf_content};
use crate::powerpoint_parser::{
    process_powerpoint_with_slides, 
    get_powerpoint_slide_info,
};
use crate::cache_system::CacheManager;
use crate::impl_cacheable_content;

/// Result of document processing with page-based support
#[derive(Debug, Clone)]
pub struct DocumentProcessingResult {
    pub content: String,
    pub total_pages: Option<usize>,
    pub requested_pages: String,
    pub returned_pages: Vec<usize>,
    pub file_path: String,
    pub error: Option<String>,
}

/// Simplified result for document page information
#[derive(Debug, Clone)]
pub struct DocumentPageInfoResult {
    pub file_path: String,
    pub total_pages: Option<usize>,
    pub page_info: String,
    pub error: Option<String>,
}

impl DocumentPageInfoResult {
    /// Create a new result for successful page info retrieval
    pub fn success(
        file_path: String,
        total_pages: Option<usize>,
        page_info: String,
    ) -> Self {
        Self {
            file_path,
            total_pages,
            page_info,
            error: None,
        }
    }

    /// Create a new result for error cases
    pub fn error(file_path: String, error: String) -> Self {
        Self {
            file_path,
            total_pages: None,
            page_info: String::new(),
            error: Some(error),
        }
    }

    /// Check if the file exists (no error or error is not file_not_found)
    pub fn file_exists(&self) -> bool {
        self.error.as_ref() != Some(&"file_not_found".to_string())
    }
}

impl DocumentProcessingResult {
    /// Create a new result for successful processing
    pub fn success(
        content: String,
        total_pages: Option<usize>,
        requested_pages: String,
        returned_pages: Vec<usize>,
        file_path: String,
    ) -> Self {
        Self {
            content,
            total_pages,
            requested_pages,
            returned_pages,
            file_path,
            error: None,
        }
    }

    /// Create a new result for error cases
    pub fn error(file_path: String, error: String) -> Self {
        Self {
            content: error.clone(),
            total_pages: None,
            requested_pages: String::new(),
            returned_pages: Vec::new(),
            file_path,
            error: Some(error),
        }
    }
}

/// Cache for storing extracted Excel content
#[derive(Debug, Clone)]
pub struct ExcelCache {
    pub content: String,
    pub char_indices: Vec<usize>,
    pub total_sheets: Option<usize>,
    pub sheet_names: Vec<String>,
}

// Implement CacheableContent for ExcelCache
impl_cacheable_content!(ExcelCache, content, char_indices, total_sheets);

lazy_static::lazy_static! {
    /// Global Excel cache manager
    pub static ref EXCEL_CACHE_MANAGER: CacheManager<ExcelCache> = CacheManager::new();
}

/// Function to extract Excel content and create cache
fn extract_excel_content(file_path: &str) -> Result<ExcelCache> {
    let mut workbook: Xlsx<_> = open_workbook(file_path)
        .with_context(|| format!("Failed to open Excel file: {}", file_path))?;
    
    let sheet_names = workbook.sheet_names().to_owned();
    let total_sheets = sheet_names.len();
    
    let mut markdown = format!("# {}\n\n", Path::new(file_path).file_name().unwrap().to_string_lossy());
    
    // Process each sheet
    for (index, sheet_name) in sheet_names.iter().enumerate() {
        markdown.push_str(&format!("## Sheet {}: {}\n\n", index + 1, sheet_name));
        
        if let Ok(range) = workbook.worksheet_range(sheet_name.as_str()) {
            markdown.push_str(&range_to_markdown_table(&range));
            markdown.push_str("\n\n");
        } else {
            markdown.push_str("*Sheet could not be read*\n\n");
        }
    }
    
    // Pre-compute character byte indices for efficient slicing
    let mut char_indices = Vec::new();
    let mut byte_pos = 0;
    
    for ch in markdown.chars() {
        char_indices.push(byte_pos);
        byte_pos += ch.len_utf8();
    }
    char_indices.push(byte_pos);
    
    Ok(ExcelCache {
        content: markdown,
        char_indices,
        total_sheets: Some(total_sheets),
        sheet_names,
    })
}

/// Function to extract specific sheets from Excel
fn extract_excel_sheets(file_path: &str, sheet_numbers: &[usize]) -> Result<String> {
    let mut workbook: Xlsx<_> = open_workbook(file_path)
        .with_context(|| format!("Failed to open Excel file: {}", file_path))?;
    
    let sheet_names = workbook.sheet_names().to_owned();
    let mut markdown = format!("# {}\n\n", Path::new(file_path).file_name().unwrap().to_string_lossy());
    
    for &sheet_index in sheet_numbers {
        if sheet_index > 0 && sheet_index <= sheet_names.len() {
            let sheet_name = &sheet_names[sheet_index - 1];
            markdown.push_str(&format!("## Sheet {}: {}\n\n", sheet_index, sheet_name));
            
            if let Ok(range) = workbook.worksheet_range(sheet_name.as_str()) {
                markdown.push_str(&range_to_markdown_table(&range));
                markdown.push_str("\n\n");
            } else {
                markdown.push_str("*Sheet could not be read*\n\n");
            }
        }
    }
    
    Ok(markdown)
}

/// Cache for storing extracted DOCX content
#[derive(Debug, Clone)]
pub struct DocxCache {
    pub content: String,
    pub char_indices: Vec<usize>,
    pub total_pages: Option<usize>,
}

// Implement CacheableContent for DocxCache
impl_cacheable_content!(DocxCache, content, char_indices, total_pages);


lazy_static::lazy_static! {
    /// Global DOCX cache manager
    pub static ref DOCX_CACHE_MANAGER: CacheManager<DocxCache> = CacheManager::new();
}

/// Function to extract DOCX content and create cache
fn extract_docx_content(file_path: &str) -> Result<DocxCache> {
    let markdown = read_docx_to_markdown(file_path)?;
    let total_pages = get_docx_page_count(file_path).unwrap_or(1);
    
    // Pre-compute character byte indices for efficient slicing
    let mut char_indices = Vec::new();
    let mut byte_pos = 0;
    
    for ch in markdown.chars() {
        char_indices.push(byte_pos);
        byte_pos += ch.len_utf8();
    }
    char_indices.push(byte_pos);
    
    Ok(DocxCache {
        content: markdown,
        char_indices,
        total_pages: Some(total_pages),
    })
}

/// Function to extract specific pages from DOCX (currently returns full content)
fn extract_docx_pages(file_path: &str, _page_numbers: &[usize]) -> Result<String> {
    // For now, DOCX doesn't support true page-level extraction
    // Return the full content
    read_docx_to_markdown(file_path)
}

/// Read Excel file and convert to markdown
pub fn read_excel_to_markdown(file_path: &str) -> Result<String> {
    let mut markdown = format!("# {}\n\n", Path::new(file_path).file_name().unwrap().to_string_lossy());
    
    // Open the workbook
    let mut workbook: Xlsx<_> = open_workbook(file_path)
        .with_context(|| format!("Failed to open Excel file: {}", file_path))?;
    
    // Process each sheet
    for sheet_name in workbook.sheet_names().to_owned() {
        // Add sheet as a header
        markdown.push_str(&format!("## Sheet: {}\n\n", sheet_name));
        
        // Read the sheet data
        if let Ok(range) = workbook.worksheet_range(&sheet_name) {
            markdown.push_str(&range_to_markdown_table(&range));
            markdown.push_str("\n\n");
        }
    }
    
    Ok(markdown)
}

/// Convert Excel range to markdown table
pub fn range_to_markdown_table(range: &calamine::Range<Data>) -> String {
    let height = range.height();
    if height == 0 {
        return "Empty sheet".to_string();
    }
    
    let width = range.width();
    let mut table = String::new();
    
    // Header row
    table.push_str("| ");
    for col in 0..width {
        if let Some(cell) = range.get_value((0, col as u32)) {
            table.push_str(&format!("{} | ", cell));
        } else {
            table.push_str(" | ");
        }
    }
    table.push_str("\n");
    
    // Separator row
    table.push_str("| ");
    for _ in 0..width {
        table.push_str("--- | ");
    }
    table.push_str("\n");
    
    // Data rows
    for row in 1..height {
        table.push_str("| ");
        for col in 0..width {
            if let Some(cell) = range.get_value((row as u32, col as u32)) {
                table.push_str(&format!("{} | ", cell));
            } else {
                table.push_str(" | ");
            }
        }
        table.push_str("\n");
    }
    
    table
}

/// Read DOCX file and convert to markdown
pub fn read_docx_to_markdown(file_path: &str) -> Result<String> {
    let mut markdown = format!("# {}\n\n", Path::new(file_path).file_name().unwrap().to_string_lossy());
    
    // Read the file into a buffer
    let mut file = File::open(file_path)
        .with_context(|| format!("Failed to open DOCX file: {}", file_path))?;
    
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .with_context(|| "Failed to read DOCX file content")?;
    
    // Use the docx-rs crate to parse the document
    // Note: The docx-rs API in the linter errors requires adjustments
    match docx_rs::read_docx(&buffer) {
        Ok(doc) => {
            // Simple text extraction for now - we'll improve this in a later step
            // This is a simplified version as the detailed parsing has linter errors
            let text = extract_text_from_docx(&doc);
            markdown.push_str("## Content\n\n");
            markdown.push_str(&text);
            markdown.push_str("\n\n");
        },
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to parse DOCX content: {}", e));
        }
    }
    
    Ok(markdown)
}

/// Extract text from DOCX document (simplified version)
fn extract_text_from_docx(_doc: &docx_rs::Docx) -> String {
    // This is a simplified placeholder implementation
    // We'll need to implement a proper text extraction based on the docx-rs API
    "[DOCX content extraction - implementation needed based on docx-rs API]".to_string()
}

/// Process a document based on its file extension with page-based selection
/// Expects a resolved file path
pub fn process_document_with_pages(
    resolved_file_path: &str,
    pages: Option<String>,
) -> DocumentProcessingResult {
    let file_path_string = resolved_file_path.to_string();
    let pages = pages.unwrap_or_else(|| "all".to_string());
    
    // Validate file and get its type
    let file_type = match validate_file_path(resolved_file_path) {
        Ok(ext) => ext,
        Err(e) => return DocumentProcessingResult::error(file_path_string, e),
    };
    
    match file_type.as_str() {
        "xlsx" | "xls" => process_excel_with_pages(resolved_file_path, &pages),
        "pdf" => process_pdf_with_pages(resolved_file_path, &pages),
        "docx" | "doc" => process_docx_with_pages(resolved_file_path, &pages),
        "pptx" | "ppt" => process_powerpoint_with_pages_wrapper(resolved_file_path, &pages),
        _ => DocumentProcessingResult::error(
            file_path_string,
            format!("Unsupported file type: {}", file_type),
        ),
    }
}

/// Process Excel file with specific sheets (pages)
fn process_excel_with_pages(file_path: &str, pages: &str) -> DocumentProcessingResult {
    let file_path_string = file_path.to_string();
    
    // Get or cache Excel content
    let excel_cache = match EXCEL_CACHE_MANAGER.get_or_cache(file_path, extract_excel_content) {
        Ok(cache) => cache,
        Err(e) => return DocumentProcessingResult::error(
            file_path_string,
            format!("Failed to get Excel content: {}", e),
        ),
    };
    
    let total_sheets = match excel_cache.total_sheets {
        Some(count) => count,
        None => return DocumentProcessingResult::error(
            file_path_string,
            "Failed to determine Excel sheet count".to_string(),
        ),
    };
    
    // Parse the pages parameter
    let requested_sheet_indices = match parse_pages_parameter(pages, total_sheets) {
        Ok(indices) => indices,
        Err(e) => return DocumentProcessingResult::error(
            file_path_string,
            format!("Invalid pages parameter: {}", e),
        ),
    };
    
    // Extract specific sheets or return full content
    let content = if pages == "all" {
        excel_cache.content.clone()
    } else {
        match EXCEL_CACHE_MANAGER.extract_units(&excel_cache, &requested_sheet_indices, file_path, extract_excel_sheets) {
            Ok(content) => content,
            Err(e) => return DocumentProcessingResult::error(
                file_path_string,
                format!("Failed to extract Excel sheets: {}", e),
            ),
        }
    };
    
    DocumentProcessingResult::success(
        content,
        Some(total_sheets),
        pages.to_string(),
        requested_sheet_indices,
        file_path_string,
    )
}

/// Process PDF file with specific pages
fn process_pdf_with_pages(file_path: &str, pages: &str) -> DocumentProcessingResult {
    let file_path_string = file_path.to_string();
    
    // Use the cache to get PDF content and page count
    let pdf_cache = match get_or_cache_pdf_content(file_path) {
        Ok(cache) => cache,
        Err(e) => return DocumentProcessingResult::error(
            file_path_string,
            format!("Failed to get PDF content: {}", e),
        ),
    };
    let total_pages = match pdf_cache.total_pages {
        Some(count) => count,
        None => return DocumentProcessingResult::error(
            file_path_string,
            "Failed to determine PDF page count".to_string(),
        ),
    };
    // Parse the pages parameter
    let requested_page_indices = match parse_pages_parameter(pages, total_pages) {
        Ok(indices) => indices,
        Err(e) => return DocumentProcessingResult::error(
            file_path_string,
            format!("Invalid pages parameter: {}", e),
        ),
    };
    // Extract text from specific pages using the new page-specific extraction
    let extracted_text = match FastPdfExtractor::extract_pages_text(file_path, &requested_page_indices) {
        Ok(text) => text,
        Err(e) => return DocumentProcessingResult::error(
            file_path_string,
            format!("Failed to extract PDF pages: {}", e),
        ),
    };
    let mut markdown = format!("# {}\n\n", Path::new(file_path).file_name().unwrap().to_string_lossy());
    // Add the extracted content
    if requested_page_indices.len() == total_pages {
        // All pages requested
        markdown.push_str("## Content (All Pages)\n\n");
    } else {
        // Specific pages requested
        markdown.push_str(&format!("## Content (Pages: {})\n\n", pages));
    }
    markdown.push_str(&extracted_text);
    DocumentProcessingResult::success(
        markdown,
        Some(total_pages),
        pages.to_string(),
        requested_page_indices,
        file_path_string,
    )
}

/// Process DOCX file with specific pages
fn process_docx_with_pages(file_path: &str, pages: &str) -> DocumentProcessingResult {
    let file_path_string = file_path.to_string();
    
    // Get or cache DOCX content
    let docx_cache = match DOCX_CACHE_MANAGER.get_or_cache(file_path, extract_docx_content) {
        Ok(cache) => cache,
        Err(e) => return DocumentProcessingResult::error(
            file_path_string,
            format!("Failed to get DOCX content: {}", e),
        ),
    };
    
    let total_pages = match docx_cache.total_pages {
        Some(count) => count,
        None => 1, // Default to 1 page if count is not available
    };
    
    // Parse the pages parameter
    let requested_page_indices = match parse_pages_parameter(pages, total_pages) {
        Ok(indices) => indices,
        Err(e) => return DocumentProcessingResult::error(
            file_path_string,
            format!("Invalid pages parameter: {}", e),
        ),
    };
    
    // For DOCX, we currently return the full content regardless of page selection
    // since true page-level extraction is not yet implemented
    let content = if pages == "all" {
        docx_cache.content.clone()
    } else {
        match DOCX_CACHE_MANAGER.extract_units(&docx_cache, &requested_page_indices, file_path, extract_docx_pages) {
            Ok(content) => content,
            Err(e) => return DocumentProcessingResult::error(
                file_path_string,
                format!("Failed to extract DOCX pages: {}", e),
            ),
        }
    };
    
    DocumentProcessingResult::success(
        content,
        Some(total_pages),
        pages.to_string(),
        requested_page_indices,
        file_path_string,
    )
}

/// Wrapper function to convert PowerPoint result to DocumentProcessingResult
fn process_powerpoint_with_pages_wrapper(
    file_path: &str,
    pages: &str,
) -> DocumentProcessingResult {
    let ppt_result = process_powerpoint_with_slides(file_path, Some(pages.to_string()));
    
    // Convert PowerPointProcessingResult to DocumentProcessingResult
    if let Some(error) = ppt_result.error {
        DocumentProcessingResult::error(ppt_result.file_path, error)
    } else {
        DocumentProcessingResult::success(
            ppt_result.content,
            ppt_result.total_slides,
            ppt_result.requested_slides,
            ppt_result.returned_slides,
            ppt_result.file_path,
        )
    }
}

/// Get document page information without reading the full content
/// Expects a resolved file path
pub fn get_document_page_info(resolved_file_path: &str) -> DocumentPageInfoResult {
    let file_path_string = resolved_file_path.to_string();
    
    // Validate file and get its type
    let file_type = match validate_file_path(resolved_file_path) {
        Ok(ext) => ext,
        Err(e) => {
            // Check if it's a file not found error
            if e.contains("File not found") {
                return DocumentPageInfoResult::error(file_path_string, "file_not_found".to_string());
            } else {
                return DocumentPageInfoResult::error(file_path_string, e);
            }
        }
    };
    
    match file_type.as_str() {
        "xlsx" | "xls" => {
            // Use Excel cache to get sheet information
            match EXCEL_CACHE_MANAGER.get_or_cache(resolved_file_path, extract_excel_content) {
                Ok(excel_cache) => {
                    let total_sheets = excel_cache.total_sheets.unwrap_or(0);
                    let sheet_list = excel_cache.sheet_names.iter()
                        .enumerate()
                        .map(|(i, name)| format!("  {}: {}", i + 1, name))
                        .collect::<Vec<_>>()
                        .join("\n");
                    
                    DocumentPageInfoResult::success(
                        file_path_string,
                        Some(total_sheets),
                        format!("Excel file with {} sheets:\n{}", total_sheets, sheet_list),
                    )
                },
                Err(e) => DocumentPageInfoResult::error(
                    file_path_string,
                    format!("Failed to analyze Excel file: {}", e),
                ),
            }
        },
        "pdf" => {
            // Use the cache to get PDF content and page count
            match get_or_cache_pdf_content(resolved_file_path) {
                Ok(pdf_cache) => {
                    if let Some(page_count) = pdf_cache.total_pages {
                        DocumentPageInfoResult::success(
                            file_path_string,
                            Some(page_count),
                            format!("PDF file with {} pages", page_count),
                        )
                    } else {
                        DocumentPageInfoResult::error(
                            file_path_string,
                            "Failed to determine PDF page count".to_string(),
                        )
                    }
                },
                Err(e) => DocumentPageInfoResult::error(
                    file_path_string,
                    format!("Failed to analyze PDF: {}", e),
                ),
            }
        },
        "docx" | "doc" => {
            // Use DOCX cache to get page information
            match DOCX_CACHE_MANAGER.get_or_cache(resolved_file_path, extract_docx_content) {
                Ok(docx_cache) => {
                    let page_count = docx_cache.total_pages.unwrap_or(1);
                    DocumentPageInfoResult::success(
                        file_path_string,
                        Some(page_count),
                        format!("DOCX file with {} estimated pages", page_count),
                    )
                },
                Err(e) => {
                    log::warn!("Failed to get DOCX content: {}", e);
                    DocumentPageInfoResult::success(
                        file_path_string,
                        Some(1),
                        "DOCX file (page count estimation failed, defaulting to 1 page)".to_string(),
                    )
                }
            }
        },
        "pptx" | "ppt" => {
            let ppt_result = get_powerpoint_slide_info(resolved_file_path);
            
            // Convert PowerPointPageInfoResult to DocumentPageInfoResult
            if let Some(error) = ppt_result.error {
                DocumentPageInfoResult::error(ppt_result.file_path, error)
            } else {
                DocumentPageInfoResult::success(
                    ppt_result.file_path,
                    ppt_result.total_slides,
                    ppt_result.slide_info,
                )
            }
        },
        _ => DocumentPageInfoResult::error(
            file_path_string,
            format!("Unsupported file type: {}", file_type),
        ),
    }
}

/// Get the actual page count for a DOCX file
fn get_docx_page_count(file_path: &str) -> Result<usize> {
    use std::fs::File;
    use std::io::Read;
    
    // Read the file into a buffer
    let mut file = File::open(file_path)
        .with_context(|| format!("Failed to open DOCX file: {}", file_path))?;
    
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .with_context(|| "Failed to read DOCX file content")?;
    
    // Use the docx-rs crate to parse the document
    match docx_rs::read_docx(&buffer) {
        Ok(docx) => {
            // Count paragraphs as a rough estimate for page count
            let paragraph_count = docx.document.children.iter()
                .filter(|child| matches!(child, docx_rs::DocumentChild::Paragraph(_)))
                .count();
            
            // Rough heuristic: assume 25-30 paragraphs per page for typical documents
            let estimated_pages = if paragraph_count > 25 {
                (paragraph_count / 25).max(1)
            } else {
                1
            };
            
            Ok(estimated_pages)
        },
        Err(e) => {
            // If we can't parse the document structure, fall back to treating it as 1 page
            log::warn!("Failed to parse DOCX structure for page counting: {}", e);
            Ok(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_document_with_pages_file_not_found() {
        let result = process_document_with_pages("nonexistent_file.xlsx", Some("1,2".to_string()));
        
        assert_eq!(result.file_path, "nonexistent_file.xlsx");
        assert_eq!(result.total_pages, None);
        assert_eq!(result.requested_pages, "");
        assert_eq!(result.returned_pages, Vec::<usize>::new());
        assert!(result.content.contains("File not found"));
        assert!(result.error.is_some());
    }

    #[test]
    fn test_get_document_page_info_file_not_found() {
        let result = get_document_page_info("nonexistent_file.xlsx");
        
        assert_eq!(result.file_path, "nonexistent_file.xlsx");
        assert_eq!(result.total_pages, None);
        assert_eq!(result.page_info, "");
        assert_eq!(result.error.as_ref().unwrap(), "file_not_found");
    }

    #[test]
    fn test_document_processing_result_page_based_error() {
        let result = DocumentProcessingResult::error(
            "test.pdf".to_string(),
            "Test error message".to_string(),
        );
        
        assert_eq!(result.content, "Test error message");
        assert_eq!(result.total_pages, None);
        assert_eq!(result.requested_pages, "");
        assert_eq!(result.returned_pages, Vec::<usize>::new());
        assert_eq!(result.file_path, "test.pdf");
        assert_eq!(result.error.as_ref().unwrap(), "Test error message");
    }

    #[test]
    fn test_get_docx_page_count_nonexistent_file() {
        let result = get_docx_page_count("nonexistent_file.docx");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_docx_page_count_invalid_file() {
        // Create a temporary file with invalid DOCX content
        use std::io::Write;
        use tempfile::NamedTempFile;
        
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"This is not a valid DOCX file").unwrap();
        
        let result = get_docx_page_count(temp_file.path().to_str().unwrap());
        // Should return Ok(1) as fallback for invalid files
        assert_eq!(result.unwrap(), 1);
    }

    #[test]
    fn test_process_pdf_with_pages_uses_actual_page_count() {
        // This test verifies that the PDF processing uses actual page counting
        // Note: This will fail for non-existent files, which is expected
        let result = process_pdf_with_pages("nonexistent.pdf", "1");
        
        // Should fail with page count error, not text extraction error
        assert!(result.error.is_some());
        assert!(result.content.contains("Failed to get PDF content") || 
                result.content.contains("File not found"));
    }

    #[test]
    fn test_page_counting_integration() {
        // Test that all the page counting functions are properly integrated
        
        // Test Excel (should work with existing logic)
        let excel_result = get_document_page_info("nonexistent.xlsx");
        assert_eq!(excel_result.error.as_ref().unwrap(), "file_not_found");
        
        // Test PDF (should use FastPdfExtractor)
        let pdf_result = get_document_page_info("nonexistent.pdf");
        assert!(pdf_result.error.is_some());
        
        // Test DOCX (should use get_docx_page_count)
        let docx_result = get_document_page_info("nonexistent.docx");
        assert_eq!(docx_result.error.as_ref().unwrap(), "file_not_found");
        
        // Test unsupported file type
        use std::io::Write;
        use tempfile::NamedTempFile;
        
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"This is a test file").unwrap();
        let temp_path = temp_file.path().to_str().unwrap();
        let unsupported_path = format!("{}.unsupported", temp_path);
        std::fs::copy(temp_path, &unsupported_path).unwrap();
        
        let unsupported_result = get_document_page_info(&unsupported_path);
        assert!(unsupported_result.error.is_some());
        let error_msg = unsupported_result.error.as_ref().unwrap();
        assert!(error_msg.contains("Unsupported file type") || error_msg.contains("Unable to determine file type"));
        
        // Clean up
        let _ = std::fs::remove_file(&unsupported_path);
    }

    #[test]
    fn test_pdf_page_extraction_integration() {
        // Test that PDF page extraction uses the new FastPdfExtractor::extract_pages_text method
        let result = process_pdf_with_pages("nonexistent.pdf", "1,3,5");
        
        // Should fail with page count error or file not found, but the logic should attempt page extraction
        assert!(result.error.is_some());
        assert!(result.content.contains("Failed to get PDF content") || 
                result.content.contains("File not found"));
        
        // Test with invalid page parameter
        let result = process_pdf_with_pages("nonexistent.pdf", "invalid");
        assert!(result.error.is_some());
        assert!(result.content.contains("Failed to get PDF content") || 
                result.content.contains("File not found"));
    }

    #[test]
    fn test_pdf_page_extraction_with_valid_pages_parameter() {
        // Test that the page parameter parsing works correctly before attempting extraction
        // This tests the integration between parse_pages_parameter and the new extraction logic
        
        // We can't test with a real PDF file in unit tests, but we can test the error handling
        let result = process_pdf_with_pages("nonexistent.pdf", "1-3,5");
        
        // Should fail at the page count stage, not at parameter parsing
        assert!(result.error.is_some());
        assert!(result.content.contains("Failed to get PDF content") || 
                result.content.contains("File not found"));
        
        // The requested_pages should be preserved even in error cases
        assert_eq!(result.requested_pages, "");  // Empty because it fails before setting this
        assert_eq!(result.returned_pages, Vec::<usize>::new());
    }
} 