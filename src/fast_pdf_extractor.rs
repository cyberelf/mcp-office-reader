use anyhow::{Result, Context};
use std::sync::Once;

#[cfg(feature = "pdfium")]
use pdfium_render::prelude::*;

#[allow(dead_code)]
static PDFIUM_INIT: Once = Once::new();
#[allow(dead_code)]
static mut PDFIUM_AVAILABLE: bool = false;

/// PDF backend types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PdfBackend {
    #[cfg(feature = "pdfium")]
    Pdfium,
    #[cfg(feature = "mupdf_backend")]
    MuPDF,
    #[cfg(feature = "poppler")]
    Poppler,
    PdfExtract, // Fallback
}

/// Common trait for PDF text extraction backends
pub trait PdfExtractor {
    /// Extract all text from a PDF file
    fn extract_text(&self, file_path: &str) -> Result<String>;
    
    /// Extract text from PDF bytes
    fn extract_text_from_bytes(&self, pdf_bytes: &[u8]) -> Result<String>;
    
    /// Get the total number of pages in a PDF
    fn get_page_count(&self, file_path: &str) -> Result<usize>;
    
    /// Extract text from specific pages
    fn extract_pages_text(&self, file_path: &str, page_numbers: &[usize]) -> Result<String>;
    
    /// Get backend type
    fn backend_type(&self) -> PdfBackend;
    
    /// Get a description of this backend
    fn description(&self) -> &'static str;
}

/// Pdfium PDF extractor (fastest, most reliable)
#[cfg(feature = "pdfium")]
pub struct PdfiumExtractor {
    pub pdfium: Pdfium,
}

#[cfg(feature = "pdfium")]
impl PdfExtractor for PdfiumExtractor {
    fn extract_text(&self, file_path: &str) -> Result<String> {        
        let document = self.pdfium.load_pdf_from_file(file_path, None)
            .with_context(|| format!("Failed to load PDF with Pdfium: {}", file_path))?;
        
        let mut text = String::new();
        for page in document.pages().iter() {
            let page_text = page.text()
                .with_context(|| "Failed to extract text from page with Pdfium")?;
            text.push_str(&page_text.all());
            text.push('\n');
        }
        
        Ok(text)
    }
    
    fn extract_text_from_bytes(&self, pdf_bytes: &[u8]) -> Result<String> {
        let document = self.pdfium.load_pdf_from_byte_slice(pdf_bytes, None)
            .with_context(|| "Failed to load PDF from bytes with Pdfium")?;
        
        let mut text = String::new();
        for page in document.pages().iter() {
            let page_text = page.text()
                .with_context(|| "Failed to extract text from page with Pdfium")?;
            text.push_str(&page_text.all());
            text.push('\n');
        }
        
        Ok(text)
    }
    
    fn get_page_count(&self, file_path: &str) -> Result<usize> {
        let document = self.pdfium.load_pdf_from_file(file_path, None)
            .with_context(|| format!("Failed to load PDF with Pdfium: {}", file_path))?;
        
        Ok(document.pages().len() as usize)
    }
    
    fn extract_pages_text(&self, file_path: &str, page_numbers: &[usize]) -> Result<String> {
        let document = self.pdfium.load_pdf_from_file(file_path, None)
            .with_context(|| format!("Failed to load PDF with Pdfium: {}", file_path))?;
        
        let total_pages = document.pages().len() as usize;
        let mut text = String::new();
        
        // Validate page numbers first
        for &page_num in page_numbers {
            if page_num == 0 || page_num > total_pages {
                return Err(anyhow::anyhow!("Page {} is out of range (1-{})", page_num, total_pages));
            }
        }
        
        // Collect all pages and then extract the requested ones
        let all_pages: Vec<_> = document.pages().iter().collect();
        
        for &page_num in page_numbers {
            let page = &all_pages[page_num - 1]; // Convert to 0-based index
            
            let page_text = page.text()
                .with_context(|| format!("Failed to extract text from page {} with Pdfium", page_num))?;
            
            text.push_str(&format!("=== Page {} ===\n", page_num));
            text.push_str(&page_text.all());
            text.push_str("\n\n");
        }
        
        Ok(text)
    }
    
    fn backend_type(&self) -> PdfBackend {
        PdfBackend::Pdfium
    }
    
    fn description(&self) -> &'static str {
        "Google Pdfium (fastest, most reliable)"
    }
}

#[cfg(feature = "pdfium")]
impl PdfiumExtractor {
    pub fn new() -> Self {
        Self {
            pdfium: Pdfium::new(
                Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path("./"))
                .or_else(|_| Pdfium::bind_to_system_library())
                .unwrap()
            ),
        }
    }

}

/// MuPDF extractor (very fast for large files)
#[cfg(feature = "mupdf_backend")]
pub struct MuPdfExtractor;

#[cfg(feature = "mupdf_backend")]
impl PdfExtractor for MuPdfExtractor {
    fn extract_text(&self, file_path: &str) -> Result<String> {
        use mupdf::Document;
        
        let doc = Document::open(file_path)
            .with_context(|| format!("Failed to load PDF with MuPDF: {}", file_path))?;
        
        let mut text = String::new();
        let page_count = doc.page_count()
            .with_context(|| "Failed to get page count with MuPDF")?;
        
        for page_num in 0..page_count {
            let page = doc.load_page(page_num)
                .with_context(|| format!("Failed to load page {} with MuPDF", page_num))?;
            
            let page_text = page.to_text()
                .with_context(|| format!("Failed to extract text from page {} with MuPDF", page_num))?;
            
            text.push_str(&page_text);
            text.push('\n');
        }
        
        Ok(text)
    }
    
    fn extract_text_from_bytes(&self, pdf_bytes: &[u8]) -> Result<String> {
        use mupdf::Document;
        
        let doc = Document::from_bytes(pdf_bytes)
            .with_context(|| "Failed to load PDF from bytes with MuPDF")?;
        
        let mut text = String::new();
        let page_count = doc.page_count()
            .with_context(|| "Failed to get page count with MuPDF")?;
        
        for page_num in 0..page_count {
            let page = doc.load_page(page_num)
                .with_context(|| format!("Failed to load page {} with MuPDF", page_num))?;
            
            let page_text = page.to_text()
                .with_context(|| format!("Failed to extract text from page {} with MuPDF", page_num))?;
            
            text.push_str(&page_text);
            text.push('\n');
        }
        
        Ok(text)
    }
    
    fn get_page_count(&self, file_path: &str) -> Result<usize> {
        use mupdf::Document;
        
        let doc = Document::open(file_path)
            .with_context(|| format!("Failed to load PDF with MuPDF: {}", file_path))?;
        
        let page_count = doc.page_count()
            .with_context(|| "Failed to get page count with MuPDF")?;
        
        Ok(page_count as usize)
    }
    
    fn extract_pages_text(&self, file_path: &str, page_numbers: &[usize]) -> Result<String> {
        use mupdf::Document;
        
        let doc = Document::open(file_path)
            .with_context(|| format!("Failed to load PDF with MuPDF: {}", file_path))?;
        
        let total_pages = doc.page_count()
            .with_context(|| "Failed to get page count with MuPDF")? as usize;
        
        let mut text = String::new();
        
        for &page_num in page_numbers {
            if page_num == 0 || page_num > total_pages {
                return Err(anyhow::anyhow!("Page {} is out of range (1-{})", page_num, total_pages));
            }
            
            let page = doc.load_page((page_num - 1) as i32) // Convert to 0-based index
                .with_context(|| format!("Failed to load page {} with MuPDF", page_num))?;
            
            let page_text = page.to_text()
                .with_context(|| format!("Failed to extract text from page {} with MuPDF", page_num))?;
            
            text.push_str(&format!("=== Page {} ===\n", page_num));
            text.push_str(&page_text);
            text.push_str("\n\n");
        }
        
        Ok(text)
    }
    
    fn backend_type(&self) -> PdfBackend {
        PdfBackend::MuPDF
    }
    
    fn description(&self) -> &'static str {
        "MuPDF (very fast for large files)"
    }
}

/// Poppler extractor (fast, good compatibility)
#[cfg(feature = "poppler")]
pub struct PopplerExtractor;

#[cfg(feature = "poppler")]
impl PdfExtractor for PopplerExtractor {
    fn extract_text(&self, file_path: &str) -> Result<String> {
        use poppler_rs::PopplerDocument;
        
        let doc = PopplerDocument::new_from_file(file_path, "")
            .with_context(|| format!("Failed to load PDF with Poppler: {}", file_path))?;
        
        let mut text = String::new();
        let page_count = doc.get_n_pages();
        
        for page_num in 0..page_count {
            let page = doc.get_page(page_num)
                .with_context(|| format!("Failed to load page {} with Poppler", page_num))?;
            
            let page_text = page.get_text()
                .with_context(|| format!("Failed to extract text from page {} with Poppler", page_num))?;
            
            text.push_str(&page_text);
            text.push('\n');
        }
        
        Ok(text)
    }
    
    fn extract_text_from_bytes(&self, pdf_bytes: &[u8]) -> Result<String> {
        use poppler_rs::PopplerDocument;
        
        let doc = PopplerDocument::new_from_data(pdf_bytes, "")
            .with_context(|| "Failed to load PDF from bytes with Poppler")?;
        
        let mut text = String::new();
        let page_count = doc.get_n_pages();
        
        for page_num in 0..page_count {
            let page = doc.get_page(page_num)
                .with_context(|| format!("Failed to load page {} with Poppler", page_num))?;
            
            let page_text = page.get_text()
                .with_context(|| format!("Failed to extract text from page {} with Poppler", page_num))?;
            
            text.push_str(&page_text);
            text.push('\n');
        }
        
        Ok(text)
    }
    
    fn get_page_count(&self, file_path: &str) -> Result<usize> {
        use poppler_rs::PopplerDocument;
        
        let doc = PopplerDocument::new_from_file(file_path, "")
            .with_context(|| format!("Failed to load PDF with Poppler: {}", file_path))?;
        
        Ok(doc.get_n_pages() as usize)
    }
    
    fn extract_pages_text(&self, file_path: &str, page_numbers: &[usize]) -> Result<String> {
        use poppler_rs::PopplerDocument;
        
        let doc = PopplerDocument::new_from_file(file_path, "")
            .with_context(|| format!("Failed to load PDF with Poppler: {}", file_path))?;
        
        let total_pages = doc.get_n_pages() as usize;
        let mut text = String::new();
        
        for &page_num in page_numbers {
            if page_num == 0 || page_num > total_pages {
                return Err(anyhow::anyhow!("Page {} is out of range (1-{})", page_num, total_pages));
            }
            
            let page = doc.get_page((page_num - 1) as i32) // Convert to 0-based index
                .with_context(|| format!("Failed to load page {} with Poppler", page_num))?;
            
            let page_text = page.get_text()
                .with_context(|| format!("Failed to extract text from page {} with Poppler", page_num))?;
            
            text.push_str(&format!("=== Page {} ===\n", page_num));
            text.push_str(&page_text);
            text.push_str("\n\n");
        }
        
        Ok(text)
    }
    
    fn backend_type(&self) -> PdfBackend {
        PdfBackend::Poppler
    }
    
    fn description(&self) -> &'static str {
        "Poppler (fast, good compatibility)"
    }
}

/// PDF Extract fallback extractor (slowest, limited encoding support)
pub struct PdfExtractExtractor;

impl PdfExtractor for PdfExtractExtractor {
    fn extract_text(&self, file_path: &str) -> Result<String> {
        match std::panic::catch_unwind(|| {
            pdf_extract::extract_text(file_path)
        }) {
            Ok(Ok(text)) => Ok(text),
            Ok(Err(e)) => Err(anyhow::anyhow!("Failed to extract text with pdf-extract: {}", e))
                .with_context(|| format!("Failed to extract text with pdf-extract: {}", file_path)),
            Err(panic_info) => {
                let panic_msg = if let Some(s) = panic_info.downcast_ref::<String>() {
                    s.clone()
                } else if let Some(s) = panic_info.downcast_ref::<&str>() {
                    s.to_string()
                } else {
                    "Unknown panic in pdf_extract::extract_text".to_string()
                };
                log::error!("🚨 PANIC caught in pdf-extract backend: {}", panic_msg);
                
                // Check if it's an encoding-related panic
                if panic_msg.contains("unsupported encoding") || panic_msg.contains("GBK") || panic_msg.contains("encoding") {
                    Err(anyhow::anyhow!("PDF contains unsupported text encoding ({}). This PDF uses a character encoding that the pdf-extract library cannot handle. Try using a different PDF or converting it to use standard UTF-8 encoding.", panic_msg))
                } else {
                    Err(anyhow::anyhow!("pdf-extract backend panicked: {}", panic_msg))
                }
            }
        }
    }
    
    fn extract_text_from_bytes(&self, pdf_bytes: &[u8]) -> Result<String> {
        match std::panic::catch_unwind(|| {
            pdf_extract::extract_text_from_mem(pdf_bytes)
        }) {
            Ok(Ok(text)) => Ok(text),
            Ok(Err(e)) => Err(anyhow::anyhow!("Failed to extract text from bytes with pdf-extract: {}", e))
                .with_context(|| "Failed to extract text from bytes with pdf-extract"),
            Err(panic_info) => {
                let panic_msg = if let Some(s) = panic_info.downcast_ref::<String>() {
                    s.clone()
                } else if let Some(s) = panic_info.downcast_ref::<&str>() {
                    s.to_string()
                } else {
                    "Unknown panic in pdf_extract::extract_text_from_mem".to_string()
                };
                log::error!("🚨 PANIC caught in pdf-extract backend (from bytes): {}", panic_msg);
                
                // Check if it's an encoding-related panic
                if panic_msg.contains("unsupported encoding") || panic_msg.contains("GBK") || panic_msg.contains("encoding") {
                    Err(anyhow::anyhow!("PDF contains unsupported text encoding ({}). This PDF uses a character encoding that the pdf-extract library cannot handle. Try using a different PDF or converting it to use standard UTF-8 encoding.", panic_msg))
                } else {
                    Err(anyhow::anyhow!("pdf-extract backend panicked: {}", panic_msg))
                }
            }
        }
    }
    
    fn get_page_count(&self, file_path: &str) -> Result<usize> {
        // pdf-extract doesn't have a direct page count function, so we need to extract and count
        // This is less efficient but serves as a fallback
        let text = match std::panic::catch_unwind(|| {
            pdf_extract::extract_text(file_path)
        }) {
            Ok(Ok(text)) => text,
            Ok(Err(e)) => {
                return Err(anyhow::anyhow!("Failed to extract text with pdf-extract: {}", e))
                    .with_context(|| format!("Failed to extract text with pdf-extract: {}", file_path));
            },
            Err(panic_info) => {
                let panic_msg = if let Some(s) = panic_info.downcast_ref::<String>() {
                    s.clone()
                } else if let Some(s) = panic_info.downcast_ref::<&str>() {
                    s.to_string()
                } else {
                    "Unknown panic in pdf_extract::extract_text".to_string()
                };
                log::error!("🚨 PANIC caught in pdf-extract backend (page count): {}", panic_msg);
                
                // Check if it's an encoding-related panic
                if panic_msg.contains("unsupported encoding") || panic_msg.contains("GBK") || panic_msg.contains("encoding") {
                    return Err(anyhow::anyhow!("PDF contains unsupported text encoding ({}). This PDF uses a character encoding that the pdf-extract library cannot handle. Try using a different PDF or converting it to use standard UTF-8 encoding.", panic_msg));
                } else {
                    return Err(anyhow::anyhow!("pdf-extract backend panicked: {}", panic_msg));
                }
            }
        };
        
        // Estimate page count by counting form feed characters or use a simple heuristic
        let estimated_pages = text.matches('\x0C').count().max(1); // Form feed character
        if estimated_pages == 1 {
            // If no form feeds, estimate based on text length (rough heuristic)
            let chars_per_page = 3000; // Rough estimate
            Ok((text.len() / chars_per_page).max(1))
        } else {
            Ok(estimated_pages)
        }
    }
    
    fn extract_pages_text(&self, file_path: &str, page_numbers: &[usize]) -> Result<String> {
        log::debug!("🔍 extract_pages_with_pdf_extract: ENTRY - file_path={}, page_numbers={:?}", 
                   file_path, page_numbers);
        
        // Check for potential encoding compatibility issues before attempting extraction
        match Self::check_encoding_compatibility(file_path) {
            Ok(false) => {
                log::warn!("⚠️ extract_pages_with_pdf_extract: PDF likely contains unsupported encoding, extraction may fail");
            },
            Ok(true) => {
                log::debug!("🔍 extract_pages_with_pdf_extract: PDF encoding appears compatible");
            },
            Err(e) => {
                log::debug!("🔍 extract_pages_with_pdf_extract: Could not check encoding compatibility: {}", e);
            }
        }
        
        // pdf-extract doesn't support page-specific extraction directly
        // So we extract all text and try to split by form feed characters or estimate
        log::debug!("🔍 extract_pages_with_pdf_extract: About to call pdf_extract::extract_text");
        
        let full_text = match std::panic::catch_unwind(|| {
            pdf_extract::extract_text(file_path)
        }) {
            Ok(Ok(text)) => {
                log::debug!("🔍 extract_pages_with_pdf_extract: pdf_extract::extract_text succeeded, text length={}", 
                           text.len());
                text
            },
            Ok(Err(e)) => {
                log::error!("❌ extract_pages_with_pdf_extract: pdf_extract::extract_text failed: {}", e);
                return Err(anyhow::anyhow!("Failed to extract text with pdf-extract: {}", e))
                    .with_context(|| format!("Failed to extract text with pdf-extract: {}", file_path));
            },
            Err(panic_info) => {
                let panic_msg = if let Some(s) = panic_info.downcast_ref::<String>() {
                    s.clone()
                } else if let Some(s) = panic_info.downcast_ref::<&str>() {
                    s.to_string()
                } else {
                    "Unknown panic in pdf_extract::extract_text".to_string()
                };
                
                log::error!("❌ extract_pages_with_pdf_extract: PANIC in pdf_extract::extract_text: {}", panic_msg);
                
                // Check for specific encoding issues
                if panic_msg.contains("unsupported encoding") || panic_msg.contains("GBK-EUC-H") {
                    log::warn!("⚠️ extract_pages_with_pdf_extract: PDF contains unsupported encoding ({}), returning fallback message", panic_msg);
                    return Ok(Self::create_encoding_fallback_message(file_path, page_numbers, &panic_msg));
                }
                
                return Err(anyhow::anyhow!("pdf_extract panic: {}", panic_msg));
            }
        };
        
        log::debug!("🔍 extract_pages_with_pdf_extract: Splitting text by form feed characters");
        // Try to split by form feed characters (page breaks)
        let pages: Vec<&str> = full_text.split('\x0C').collect();
        log::debug!("🔍 extract_pages_with_pdf_extract: Found {} pages after splitting", pages.len());
        
        if pages.len() > 1 {
            log::debug!("🔍 extract_pages_with_pdf_extract: Using form feed characters as page boundaries");
            // We have form feed characters, use them as page boundaries
            let mut result = String::new();
            for &page_num in page_numbers {
                log::debug!("🔍 extract_pages_with_pdf_extract: Processing page {}", page_num);
                if page_num == 0 || page_num > pages.len() {
                    log::error!("❌ extract_pages_with_pdf_extract: Page {} is out of range (1-{})", 
                               page_num, pages.len());
                    return Err(anyhow::anyhow!("Page {} is out of range (1-{})", page_num, pages.len()));
                }
                
                result.push_str(&format!("=== Page {} ===\n", page_num));
                result.push_str(pages[page_num - 1]); // Convert to 0-based index
                result.push_str("\n\n");
            }
            log::debug!("🔍 extract_pages_with_pdf_extract: SUCCESS - extracted {} pages, result length={}", 
                       page_numbers.len(), result.len());
            Ok(result)
        } else {
            log::debug!("🔍 extract_pages_with_pdf_extract: No form feed characters found, returning full text with note");
            // No clear page boundaries, return full text with a note
            // This is a limitation of the pdf-extract backend
            let mut result = String::new();
            result.push_str("=== Note: pdf-extract backend cannot extract specific pages ===\n");
            result.push_str(&format!("Requested pages: {:?}\n", page_numbers));
            result.push_str("Returning full document content:\n\n");
            result.push_str(&full_text);
            log::debug!("🔍 extract_pages_with_pdf_extract: SUCCESS - returned full text, result length={}", 
                       result.len());
            Ok(result)
        }
    }
    
    fn backend_type(&self) -> PdfBackend {
        PdfBackend::PdfExtract
    }
    
    fn description(&self) -> &'static str {
        "pdf-extract (slowest, fallback, limited encoding support)"
    }
}

impl PdfExtractExtractor {
    /// Check if a PDF might have encoding issues based on common patterns
    pub fn check_encoding_compatibility(file_path: &str) -> Result<bool> {
        // Try to quickly detect potential encoding issues by reading metadata
        // This is a heuristic approach - not 100% accurate but helps predict issues
        
        use std::fs::File;
        use std::io::Read;
        
        let mut file = File::open(file_path)?;
        let mut buffer = vec![0; 8192]; // Read first 8KB to check for encoding markers
        let bytes_read = file.read(&mut buffer)?;
        buffer.truncate(bytes_read);
        
        let content = String::from_utf8_lossy(&buffer);
        
        // Check for common CJK encoding references that cause pdf-extract issues
        let problematic_encodings = [
            "GBK-EUC-H", "GBK-EUC-V", "GB-EUC-H", "GB-EUC-V",
            "UniGB-UCS2-H", "UniGB-UCS2-V", "UniGB-UTF16-H", "UniGB-UTF16-V",
            "B5pc-H", "B5pc-V", "ETen-B5-H", "ETen-B5-V",
            "CNS-EUC-H", "CNS-EUC-V", "UniCNS-UCS2-H", "UniCNS-UCS2-V",
            "90ms-RKSJ-H", "90ms-RKSJ-V", "90msp-RKSJ-H", "90msp-RKSJ-V",
            "UniJIS-UCS2-H", "UniJIS-UCS2-V", "UniJIS-UTF16-H", "UniJIS-UTF16-V",
            "KSC-EUC-H", "KSC-EUC-V", "KSCms-UHC-H", "KSCms-UHC-V",
            "UniKS-UCS2-H", "UniKS-UCS2-V", "UniKS-UTF16-H", "UniKS-UTF16-V"
        ];
        
        for encoding in &problematic_encodings {
            if content.contains(encoding) {
                log::debug!("🔍 check_encoding_compatibility: Found potentially problematic encoding: {}", encoding);
                return Ok(false); // Likely to have encoding issues
            }
        }
        
        Ok(true) // Likely compatible
    }

    /// Create a fallback message for PDFs with unsupported encodings
    fn create_encoding_fallback_message(file_path: &str, page_numbers: &[usize], encoding_error: &str) -> String {
        let file_name = std::path::Path::new(file_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(file_path);
            
        let mut result = String::new();
        result.push_str("=== PDF Text Extraction Notice ===\n\n");
        result.push_str(&format!("File: {}\n", file_name));
        result.push_str(&format!("Requested pages: {:?}\n\n", page_numbers));
        
        result.push_str("⚠️ **Text extraction temporarily unavailable**\n\n");
        result.push_str("This PDF uses a character encoding that is not currently supported by the text extraction library:\n");
        result.push_str(&format!("- Encoding issue: {}\n\n", encoding_error));
        
        result.push_str("**What this means:**\n");
        result.push_str("- The PDF file is likely valid and can be opened in standard PDF viewers\n");
        result.push_str("- The file may contain Chinese, Japanese, or other non-Latin characters\n");
        result.push_str("- Text extraction is blocked by the encoding limitation, not a file corruption\n\n");
        
        result.push_str("**Alternatives:**\n");
        result.push_str("- Open the PDF in a standard PDF viewer to read the content\n");
        result.push_str("- Try converting the PDF to a different format that supports the encoding\n");
        result.push_str("- Use a different PDF processing tool that supports the specific encoding\n\n");
        
        result.push_str("**Technical details:**\n");
        result.push_str("- This is a known limitation of the pdf-extract library\n");
        result.push_str("- The encoding issue prevents text extraction but doesn't affect PDF validity\n");
        result.push_str(&format!("- Error: {}\n", encoding_error));
        
        result
    }
}

/// Main PDF extraction interface that selects the best available backend
pub struct FastPdfExtractor;

impl FastPdfExtractor {
    /// Get available PDF extractors in order of preference (fastest first)
    fn get_available_extractors() -> Vec<Box<dyn PdfExtractor>> {
        let mut extractors: Vec<Box<dyn PdfExtractor>> = Vec::new();
        
        // Pdfium is fastest and most reliable - but only if actually available
        #[cfg(feature = "pdfium")]
        {
            extractors.push(Box::new(PdfiumExtractor::new()));
        }
        
        // MuPDF is very fast for large files
        #[cfg(feature = "mupdf_backend")]
        extractors.push(Box::new(MuPdfExtractor));
        
        // Poppler is fast and has good compatibility
        #[cfg(feature = "poppler")]
        extractors.push(Box::new(PopplerExtractor));
        
        // pdf-extract as fallback (slowest but most compatible)
        extractors.push(Box::new(PdfExtractExtractor));
        
        extractors
    }
    
    /// Extract text from PDF file using the fastest available backend
    pub fn extract_text(file_path: &str) -> Result<String> {
        let extractors = Self::get_available_extractors();
        
        for extractor in extractors {
            match extractor.extract_text(file_path) {
                Ok(text) => return Ok(text),
                Err(e) => {
                    log::warn!("Backend {:?} failed: {}", extractor.backend_type(), e);
                    continue;
                }
            }
        }
        
        anyhow::bail!("All PDF extraction backends failed for file: {}", file_path);
    }
    
    /// Extract text from PDF bytes using the fastest available backend
    pub fn extract_text_from_bytes(pdf_bytes: &[u8]) -> Result<String> {
        let extractors = Self::get_available_extractors();
        
        for extractor in extractors {
            match extractor.extract_text_from_bytes(pdf_bytes) {
                Ok(text) => {
                    log::debug!("Successfully extracted PDF text from bytes using {:?} backend", extractor.backend_type());
                    return Ok(text);
                }
                Err(e) => {
                    log::warn!("Failed to extract PDF from bytes with {:?} backend: {}", extractor.backend_type(), e);
                    continue;
                }
            }
        }
        
        anyhow::bail!("All PDF extraction backends failed for byte array")
    }
    
    /// Get the page count of a PDF file without extracting text (more efficient)
    pub fn get_page_count(file_path: &str) -> Result<usize> {
        let extractors = Self::get_available_extractors();
        
        for extractor in extractors {
            match extractor.get_page_count(file_path) {
                Ok(count) => return Ok(count),
                Err(e) => {
                    log::warn!("Backend {:?} failed to get page count: {}", extractor.backend_type(), e);
                    continue;
                }
            }
        }
        
        anyhow::bail!("All PDF backends failed to get page count for file: {}", file_path);
    }

    /// Extract text from specific pages of a PDF file using the fastest available backend
    pub fn extract_pages_text(file_path: &str, page_numbers: &[usize]) -> Result<String> {
        log::debug!("🔍 FastPdfExtractor::extract_pages_text: ENTRY - file_path={}, page_numbers={:?}", 
                   file_path, page_numbers);
        
        let extractors = Self::get_available_extractors();
        log::debug!("🔍 FastPdfExtractor::extract_pages_text: Available extractors count: {}", extractors.len());
        
        for (index, extractor) in extractors.iter().enumerate() {
            log::debug!("🔍 FastPdfExtractor::extract_pages_text: Trying backend {} of {}: {:?}", 
                       index + 1, extractors.len(), extractor.backend_type());
            
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                extractor.extract_pages_text(file_path, page_numbers)
            })) {
                Ok(Ok(text)) => {
                    log::debug!("🔍 FastPdfExtractor::extract_pages_text: SUCCESS with backend {:?}, text length={}", 
                               extractor.backend_type(), text.len());
                    return Ok(text);
                },
                Ok(Err(e)) => {
                    log::warn!("⚠️ FastPdfExtractor::extract_pages_text: Backend {:?} failed for page extraction: {}", extractor.backend_type(), e);
                    continue;
                },
                Err(panic_info) => {
                    let panic_msg = if let Some(s) = panic_info.downcast_ref::<String>() {
                        s.clone()
                    } else if let Some(s) = panic_info.downcast_ref::<&str>() {
                        s.to_string()
                    } else {
                        "Unknown panic in PDF backend".to_string()
                    };
                    log::error!("❌ FastPdfExtractor::extract_pages_text: PANIC in backend {:?}: {}", extractor.backend_type(), panic_msg);
                    continue;
                }
            }
        }
        
        log::error!("❌ FastPdfExtractor::extract_pages_text: All backends failed");
        anyhow::bail!("All PDF extraction backends failed for page extraction from file: {}", file_path);
    }

    /// Get information about available backends
    pub fn get_backend_info() -> Vec<(PdfBackend, &'static str, bool)> {
        let extractors = Self::get_available_extractors();
        let mut info = Vec::new();
        
        for extractor in extractors {
            info.push((extractor.backend_type(), extractor.description(), true));
        }
        
        info
    }
    
    /// Check if a PDF might have encoding issues based on common patterns
    pub fn check_encoding_compatibility(file_path: &str) -> Result<bool> {
        PdfExtractExtractor::check_encoding_compatibility(file_path)
    }
}
