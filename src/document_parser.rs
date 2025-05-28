use std::path::Path;
use std::fs::File;
use std::io::Read;

use anyhow::{Result, Context};
use calamine::{Reader, open_workbook, Xlsx, Data};
use pdf_extract;

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

/// Process a document based on its file extension
pub fn process_document(file_path: &str) -> String {
    // Check if file exists
    if !Path::new(file_path).exists() {
        return format!("File not found: {}", file_path);
    }
    
    // Determine file type from extension
    let extension = Path::new(file_path)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_lowercase());
    
    match extension {
        Some(ext) => {
            match ext.as_str() {
                "xlsx" | "xls" => match read_excel_to_markdown(file_path) {
                    Ok(markdown) => markdown,
                    Err(e) => format!("Error reading Excel file: {}", e),
                },
                "pdf" => match read_pdf_to_markdown(file_path) {
                    Ok(markdown) => markdown,
                    Err(e) => format!("Error reading PDF file: {}", e),
                },
                "docx" | "doc" => match read_docx_to_markdown(file_path) {
                    Ok(markdown) => markdown,
                    Err(e) => format!("Error reading DOCX file: {}", e),
                },
                _ => format!("Unsupported file type: {}", ext),
            }
        }
        None => "Unable to determine file type (no extension)".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::write;
    use std::path::PathBuf;

    #[test]
    fn test_process_document_file_not_found() {
        let result = process_document("nonexistent_file.xlsx");
        assert!(result.contains("File not found"));
    }

    #[test]
    fn test_process_document_unsupported_type() {
        // Create a temporary text file
        let temp_file = PathBuf::from("temp_test.txt");
        write(&temp_file, "Test content").unwrap();
        
        let result = process_document(temp_file.to_str().unwrap());
        assert!(result.contains("Unsupported file type"));
        
        // Clean up
        std::fs::remove_file(temp_file).unwrap();
    }

    #[test]
    fn test_process_document_no_extension() {
        // Create a temporary file without extension
        let temp_file = PathBuf::from("temp_test_file");
        write(&temp_file, "Test content").unwrap();
        
        let result = process_document(temp_file.to_str().unwrap());
        assert!(result.contains("Unable to determine file type"));
        
        // Clean up
        std::fs::remove_file(temp_file).unwrap();
    }
} 