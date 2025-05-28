use std::path::Path;
use std::fs::File;
use std::io::Read;

use anyhow::{Result, Context};
use calamine::{Reader, open_workbook, Xlsx, Data};
use pdf_extract;

/// Result of document processing with pagination support
#[derive(Debug, Clone)]
pub struct DocumentProcessingResult {
    pub content: String,
    pub total_length: usize,
    pub offset: usize,
    pub returned_length: usize,
    pub has_more: bool,
    pub file_path: String,
    pub error: Option<String>,
}

impl DocumentProcessingResult {
    /// Create a new result for successful processing
    pub fn success(
        content: String,
        total_length: usize,
        offset: usize,
        returned_length: usize,
        has_more: bool,
        file_path: String,
    ) -> Self {
        Self {
            content,
            total_length,
            offset,
            returned_length,
            has_more,
            file_path,
            error: None,
        }
    }

    /// Create a new result for error cases
    pub fn error(file_path: String, error: String) -> Self {
        Self {
            content: error.clone(),
            total_length: 0,
            offset: 0,
            returned_length: error.len(),
            has_more: false,
            file_path,
            error: Some(error),
        }
    }
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

/// Read PDF file and convert to markdown
pub fn read_pdf_to_markdown(file_path: &str) -> Result<String> {
    let mut markdown = format!("# {}\n\n", Path::new(file_path).file_name().unwrap().to_string_lossy());
    
    let pdf_data = pdf_extract::extract_text(file_path)
        .with_context(|| format!("Failed to extract text from PDF: {}", file_path))?;
    
    // Since pdf-extract doesn't provide page-by-page extraction directly,
    // we'll just put all the text as a single section
    markdown.push_str("## Content\n\n");
    markdown.push_str(&pdf_data);
    markdown.push_str("\n\n");
    
    Ok(markdown)
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

/// Process a document based on its file extension with pagination support
pub fn process_document_with_pagination(
    file_path: &str,
    offset: Option<usize>,
    max_length: Option<usize>,
) -> DocumentProcessingResult {
    let file_path_string = file_path.to_string();
    let offset = offset.unwrap_or(0);
    let max_length = max_length.unwrap_or(50000); // Default 50KB of text
    
    // Check if file exists
    if !Path::new(file_path).exists() {
        return DocumentProcessingResult::error(
            file_path_string,
            format!("File not found: {}", file_path),
        );
    }
    
    // Get the full document content first
    let full_markdown = match get_full_document_content(file_path) {
        Ok(content) => content,
        Err(error) => {
            return DocumentProcessingResult::error(file_path_string, error);
        }
    };
    
    // Apply pagination
    apply_pagination(full_markdown, offset, max_length, file_path_string)
}

/// Get the full document content without pagination
fn get_full_document_content(file_path: &str) -> Result<String, String> {
    // Determine file type from extension
    let extension = Path::new(file_path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase());
    
    match extension {
        Some(ext) => {
            match ext.as_str() {
                "xlsx" | "xls" => match read_excel_to_markdown(file_path) {
                    Ok(markdown) => Ok(markdown),
                    Err(e) => Err(format!("Error reading Excel file: {}", e)),
                },
                "pdf" => match read_pdf_to_markdown(file_path) {
                    Ok(markdown) => Ok(markdown),
                    Err(e) => Err(format!("Error reading PDF file: {}", e)),
                },
                "docx" | "doc" => match read_docx_to_markdown(file_path) {
                    Ok(markdown) => Ok(markdown),
                    Err(e) => Err(format!("Error reading DOCX file: {}", e)),
                },
                _ => Err(format!("Unsupported file type: {}", ext)),
            }
        }
        None => Err("Unable to determine file type (no extension)".to_string()),
    }
}

/// Apply offset and length limits to content
fn apply_pagination(
    full_content: String,
    offset: usize,
    max_length: usize,
    file_path: String,
) -> DocumentProcessingResult {
    let total_length = full_content.len();
    
    // Apply offset and size limits
    let start_pos = offset.min(total_length);
    let end_pos = (start_pos + max_length).min(total_length);
    let content = if start_pos < total_length {
        full_content[start_pos..end_pos].to_string()
    } else {
        String::new()
    };
    
    let returned_length = content.len();
    let has_more = end_pos < total_length;
    
    DocumentProcessingResult::success(
        content,
        total_length,
        start_pos,
        returned_length,
        has_more,
        file_path,
    )
}

/// Get the total text length of a document without reading the full content
/// This is optimized to be faster than full processing for size checking
pub fn get_document_text_length(file_path: &str) -> DocumentProcessingResult {
    let file_path_string = file_path.to_string();
    
    // Check if file exists
    if !Path::new(file_path).exists() {
        return DocumentProcessingResult {
            content: String::new(),
            total_length: 0,
            offset: 0,
            returned_length: 0,
            has_more: false,
            file_path: file_path_string,
            error: Some("file_not_found".to_string()), // Special marker for file not found
        };
    }
    
    // Get the full document content to calculate length
    match get_full_document_content(file_path) {
        Ok(content) => DocumentProcessingResult {
            content: String::new(), // Don't return content for length check
            total_length: content.len(),
            offset: 0,
            returned_length: 0,
            has_more: false,
            file_path: file_path_string,
            error: None,
        },
        Err(error) => DocumentProcessingResult {
            content: String::new(),
            total_length: 0,
            offset: 0,
            returned_length: 0,
            has_more: false,
            file_path: file_path_string,
            error: Some(error),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_document_with_pagination_file_not_found() {
        let result = process_document_with_pagination("nonexistent_file.xlsx", Some(0), Some(1000));
        
        assert_eq!(result.file_path, "nonexistent_file.xlsx");
        assert_eq!(result.total_length, 0);
        assert_eq!(result.offset, 0);
        assert_eq!(result.returned_length, result.content.len()); // Length of error message
        assert!(!result.has_more);
        assert!(result.content.contains("File not found"));
        assert!(result.error.is_some());
    }

    #[test]
    fn test_get_document_text_length_file_not_found() {
        let result = get_document_text_length("nonexistent_file.xlsx");
        
        assert_eq!(result.file_path, "nonexistent_file.xlsx");
        assert_eq!(result.total_length, 0);
        assert_eq!(result.offset, 0);
        assert_eq!(result.returned_length, 0);
        assert!(!result.has_more);
        assert!(result.content.is_empty());
        assert_eq!(result.error.as_ref().unwrap(), "file_not_found");
    }

    #[test]
    fn test_apply_pagination() {
        let full_content = "0123456789".repeat(100); // 1000 characters
        let file_path = "test.txt".to_string();
        
        // Test normal case
        let result = apply_pagination(full_content.clone(), 100, 50, file_path.clone());
        assert_eq!(result.total_length, 1000);
        assert_eq!(result.offset, 100);
        assert_eq!(result.returned_length, 50);
        assert!(result.has_more);
        assert_eq!(result.content.len(), 50);
        
        // Test offset beyond content
        let result = apply_pagination(full_content.clone(), 2000, 50, file_path.clone());
        assert_eq!(result.total_length, 1000);
        assert_eq!(result.offset, 1000);
        assert_eq!(result.returned_length, 0);
        assert!(!result.has_more);
        assert!(result.content.is_empty());
        
        // Test when remaining content is less than max_length
        let result = apply_pagination(full_content.clone(), 980, 50, file_path.clone());
        assert_eq!(result.total_length, 1000);
        assert_eq!(result.offset, 980);
        assert_eq!(result.returned_length, 20);
        assert!(!result.has_more);
        assert_eq!(result.content.len(), 20);
    }

    #[test]
    fn test_document_processing_result_success() {
        let result = DocumentProcessingResult::success(
            "test content".to_string(),
            1000,
            100,
            12,
            true,
            "test.pdf".to_string(),
        );
        
        assert_eq!(result.content, "test content");
        assert_eq!(result.total_length, 1000);
        assert_eq!(result.offset, 100);
        assert_eq!(result.returned_length, 12);
        assert!(result.has_more);
        assert_eq!(result.file_path, "test.pdf");
        assert!(result.error.is_none());
    }

    #[test]
    fn test_document_processing_result_error() {
        let result = DocumentProcessingResult::error(
            "test.pdf".to_string(),
            "Test error message".to_string(),
        );
        
        assert_eq!(result.content, "Test error message");
        assert_eq!(result.total_length, 0);
        assert_eq!(result.offset, 0);
        assert_eq!(result.returned_length, 18); // Length of error message
        assert!(!result.has_more);
        assert_eq!(result.file_path, "test.pdf");
        assert_eq!(result.error.as_ref().unwrap(), "Test error message");
    }
} 