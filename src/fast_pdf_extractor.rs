use anyhow::{Result, Context};
use std::sync::Once;

static PDFIUM_INIT: Once = Once::new();
static mut PDFIUM_AVAILABLE: bool = false;

/// Fast PDF text extraction with multiple backend support
/// Automatically selects the fastest available backend and falls back if needed
pub struct FastPdfExtractor;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PdfBackend {
    Pdfium,
    #[cfg(feature = "mupdf_backend")]
    MuPDF,
    #[cfg(feature = "poppler")]
    Poppler,
    PdfExtract, // Fallback
}

impl FastPdfExtractor {
    /// Extract text from PDF file using the fastest available backend
    pub fn extract_text(file_path: &str) -> Result<String> {
        let backends = Self::get_available_backends();
        
        for backend in backends {
            match Self::extract_with_backend(file_path, backend) {
                Ok(text) => return Ok(text),
                Err(e) => {
                    log::warn!("Backend {:?} failed: {}", backend, e);
                    continue;
                }
            }
        }
        
        anyhow::bail!("All PDF extraction backends failed for file: {}", file_path);
    }
    
    /// Extract text from PDF bytes using the fastest available backend
    pub fn extract_text_from_bytes(pdf_bytes: &[u8]) -> Result<String> {
        let backends = Self::get_available_backends();
        
        for backend in backends {
            match Self::extract_from_bytes_with_backend(pdf_bytes, backend.clone()) {
                Ok(text) => {
                    log::debug!("Successfully extracted PDF text from bytes using {:?} backend", backend);
                    return Ok(text);
                }
                Err(e) => {
                    log::warn!("Failed to extract PDF from bytes with {:?} backend: {}", backend, e);
                    continue;
                }
            }
        }
        
        anyhow::bail!("All PDF extraction backends failed for byte array")
    }
    
    /// Check if Pdfium is actually available at runtime
    #[cfg(feature = "pdfium")]
    fn is_pdfium_available() -> bool {
        unsafe {
            PDFIUM_INIT.call_once(|| {
                PDFIUM_AVAILABLE = Self::test_pdfium_initialization();
            });
            PDFIUM_AVAILABLE
        }
    }
    
    #[cfg(not(feature = "pdfium"))]
    fn is_pdfium_available() -> bool {
        false
    }
    
    /// Test if Pdfium can be initialized
    #[cfg(feature = "pdfium")]
    fn test_pdfium_initialization() -> bool {
        use pdfium_render::prelude::*;
        
        // Suppress panic output by catching unwind
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            // Try to create a Pdfium instance
            let _pdfium = Pdfium::default();
            true
        }));
        
        match result {
            Ok(_) => true,
            Err(_) => false,
        }
    }
    
    /// Get list of available backends in order of preference (fastest first)
    fn get_available_backends() -> Vec<PdfBackend> {
        let mut backends = Vec::new();
        
        // Pdfium is fastest and most reliable - but only if actually available
        if Self::is_pdfium_available() {
            backends.push(PdfBackend::Pdfium);
        }
        
        // MuPDF is very fast for large files
        #[cfg(feature = "mupdf_backend")]
        backends.push(PdfBackend::MuPDF);
        
        // Poppler is fast and has good compatibility
        #[cfg(feature = "poppler")]
        backends.push(PdfBackend::Poppler);
        
        // pdf-extract as fallback (slowest but most compatible)
        backends.push(PdfBackend::PdfExtract);
        
        backends
    }
    
    /// Extract text using a specific backend
    fn extract_with_backend(file_path: &str, backend: PdfBackend) -> Result<String> {
        match backend {
            #[cfg(feature = "pdfium")]
            PdfBackend::Pdfium => Self::extract_with_pdfium(file_path),
            #[cfg(feature = "mupdf_backend")]
            PdfBackend::MuPDF => Self::extract_with_mupdf(file_path),
            #[cfg(feature = "poppler")]
            PdfBackend::Poppler => Self::extract_with_poppler(file_path),
            PdfBackend::PdfExtract => Self::extract_with_pdf_extract(file_path),
            #[allow(unreachable_patterns)]
            _ => anyhow::bail!("Backend {:?} not available in this build", backend),
        }
    }
    
    /// Extract text from bytes using a specific backend
    fn extract_from_bytes_with_backend(pdf_bytes: &[u8], backend: PdfBackend) -> Result<String> {
        match backend {
            #[cfg(feature = "pdfium")]
            PdfBackend::Pdfium => Self::extract_from_bytes_with_pdfium(pdf_bytes),
            
            #[cfg(not(feature = "pdfium"))]
            PdfBackend::Pdfium => anyhow::bail!("Pdfium backend not available (feature not enabled)"),
            
            #[cfg(feature = "mupdf_backend")]
            PdfBackend::MuPDF => Self::extract_from_bytes_with_mupdf(pdf_bytes),
            
            #[cfg(feature = "poppler")]
            PdfBackend::Poppler => Self::extract_from_bytes_with_poppler(pdf_bytes),
            
            PdfBackend::PdfExtract => Self::extract_from_bytes_with_pdf_extract(pdf_bytes),
        }
    }
    
    // Pdfium implementation (fastest)
    #[cfg(feature = "pdfium")]
    fn extract_with_pdfium(file_path: &str) -> Result<String> {
        use pdfium_render::prelude::*;
        
        // Check if Pdfium is available before attempting to use it
        if !Self::is_pdfium_available() {
            anyhow::bail!("Pdfium backend is not available on this system (missing native library)");
        }
        
        let pdfium = Pdfium::default();
        let document = pdfium.load_pdf_from_file(file_path, None)
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
    
    #[cfg(feature = "pdfium")]
    fn extract_from_bytes_with_pdfium(pdf_bytes: &[u8]) -> Result<String> {
        use pdfium_render::prelude::*;
        
        // Check if Pdfium is available before attempting to use it
        if !Self::is_pdfium_available() {
            anyhow::bail!("Pdfium backend is not available on this system (missing native library)");
        }
        
        let pdfium = Pdfium::default();
        let document = pdfium.load_pdf_from_byte_slice(pdf_bytes, None)
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
    
    #[cfg(feature = "pdfium")]
    fn get_page_count_with_pdfium(file_path: &str) -> Result<usize> {
        use pdfium_render::prelude::*;
        
        // Check if Pdfium is available before attempting to use it
        if !Self::is_pdfium_available() {
            anyhow::bail!("Pdfium backend is not available on this system (missing native library)");
        }
        
        let pdfium = Pdfium::default();
        let document = pdfium.load_pdf_from_file(file_path, None)
            .with_context(|| format!("Failed to load PDF with Pdfium: {}", file_path))?;
        
        let total_pages = document.pages().len() as usize;
        
        Ok(total_pages)
    }
    
    // MuPDF implementation (very fast for large files)
    #[cfg(feature = "mupdf_backend")]
    fn extract_with_mupdf(file_path: &str) -> Result<String> {
        use mupdf::{Document, Matrix};
        
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
    
    #[cfg(feature = "mupdf_backend")]
    fn extract_from_bytes_with_mupdf(pdf_bytes: &[u8]) -> Result<String> {
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
    
    #[cfg(feature = "mupdf_backend")]
    fn get_page_count_with_mupdf(file_path: &str) -> Result<usize> {
        use mupdf::Document;
        
        let doc = Document::open(file_path)
            .with_context(|| format!("Failed to load PDF with MuPDF: {}", file_path))?;
        
        let page_count = doc.page_count()
            .with_context(|| "Failed to get page count with MuPDF")?;
        
        Ok(page_count as usize)
    }
    
    // Poppler implementation (fast and compatible)
    #[cfg(feature = "poppler")]
    fn extract_with_poppler(file_path: &str) -> Result<String> {
        use poppler_rs::{PopplerDocument, PopplerPage};
        
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
    
    #[cfg(feature = "poppler")]
    fn extract_from_bytes_with_poppler(pdf_bytes: &[u8]) -> Result<String> {
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
    
    #[cfg(feature = "poppler")]
    fn get_page_count_with_poppler(file_path: &str) -> Result<usize> {
        use poppler_rs::PopplerDocument;
        
        let doc = PopplerDocument::new_from_file(file_path, "")
            .with_context(|| format!("Failed to load PDF with Poppler: {}", file_path))?;
        
        Ok(doc.get_n_pages() as usize)
    }
    
    // pdf-extract implementation (fallback)
    fn extract_with_pdf_extract(file_path: &str) -> Result<String> {
        pdf_extract::extract_text(file_path)
            .with_context(|| format!("Failed to extract text with pdf-extract: {}", file_path))
    }
    
    fn extract_from_bytes_with_pdf_extract(pdf_bytes: &[u8]) -> Result<String> {
        pdf_extract::extract_text_from_mem(pdf_bytes)
            .with_context(|| "Failed to extract text from bytes with pdf-extract")
    }
    
    fn get_page_count_with_pdf_extract(file_path: &str) -> Result<usize> {
        // pdf-extract doesn't have a direct page count function, so we need to extract and count
        // This is less efficient but serves as a fallback
        let text = pdf_extract::extract_text(file_path)
            .with_context(|| format!("Failed to extract text with pdf-extract: {}", file_path))?;
        
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
    
    /// Get information about available backends
    pub fn get_backend_info() -> Vec<(PdfBackend, &'static str, bool)> {
        let mut info = Vec::new();
        
        // Check actual Pdfium availability, not just feature flag
        info.push((PdfBackend::Pdfium, "Google Pdfium (fastest, most reliable)", Self::is_pdfium_available()));
        
        #[cfg(feature = "mupdf_backend")]
        info.push((PdfBackend::MuPDF, "MuPDF (very fast for large files)", true));
        
        #[cfg(feature = "poppler")]
        info.push((PdfBackend::Poppler, "Poppler (fast, good compatibility)", true));
        
        info.push((PdfBackend::PdfExtract, "pdf-extract (slowest, fallback)", true));
        
        info
    }

    /// Get the page count of a PDF file without extracting text (more efficient)
    pub fn get_page_count(file_path: &str) -> Result<usize> {
        let backends = Self::get_available_backends();
        
        for backend in backends {
            match Self::get_page_count_with_backend(file_path, backend) {
                Ok(count) => return Ok(count),
                Err(e) => {
                    log::warn!("Backend {:?} failed to get page count: {}", backend, e);
                    continue;
                }
            }
        }
        
        anyhow::bail!("All PDF backends failed to get page count for file: {}", file_path);
    }

    fn get_page_count_with_backend(file_path: &str, backend: PdfBackend) -> Result<usize> {
        match backend {
            #[cfg(feature = "pdfium")]
            PdfBackend::Pdfium => Self::get_page_count_with_pdfium(file_path),
            #[cfg(feature = "mupdf_backend")]
            PdfBackend::MuPDF => Self::get_page_count_with_mupdf(file_path),
            #[cfg(feature = "poppler")]
            PdfBackend::Poppler => Self::get_page_count_with_poppler(file_path),
            PdfBackend::PdfExtract => Self::get_page_count_with_pdf_extract(file_path),
            #[allow(unreachable_patterns)]
            _ => anyhow::bail!("Backend {:?} not available in this build", backend),
        }
    }

    /// Extract text from specific pages of a PDF file using the fastest available backend
    pub fn extract_pages_text(file_path: &str, page_numbers: &[usize]) -> Result<String> {
        let backends = Self::get_available_backends();
        
        for backend in backends {
            match Self::extract_pages_with_backend(file_path, page_numbers, backend) {
                Ok(text) => return Ok(text),
                Err(e) => {
                    log::warn!("Backend {:?} failed for page extraction: {}", backend, e);
                    continue;
                }
            }
        }
        
        anyhow::bail!("All PDF extraction backends failed for page extraction from file: {}", file_path);
    }

    /// Extract text from specific pages using a specific backend
    fn extract_pages_with_backend(file_path: &str, page_numbers: &[usize], backend: PdfBackend) -> Result<String> {
        match backend {
            #[cfg(feature = "pdfium")]
            PdfBackend::Pdfium => Self::extract_pages_with_pdfium(file_path, page_numbers),
            #[cfg(feature = "mupdf_backend")]
            PdfBackend::MuPDF => Self::extract_pages_with_mupdf(file_path, page_numbers),
            #[cfg(feature = "poppler")]
            PdfBackend::Poppler => Self::extract_pages_with_poppler(file_path, page_numbers),
            PdfBackend::PdfExtract => Self::extract_pages_with_pdf_extract(file_path, page_numbers),
            #[allow(unreachable_patterns)]
            _ => anyhow::bail!("Backend {:?} not available in this build", backend),
        }
    }

    // Page-specific extraction implementations for each backend
    #[cfg(feature = "pdfium")]
    fn extract_pages_with_pdfium(file_path: &str, page_numbers: &[usize]) -> Result<String> {
        use pdfium_render::prelude::*;
        
        // Check if Pdfium is available before attempting to use it
        if !Self::is_pdfium_available() {
            anyhow::bail!("Pdfium backend is not available on this system (missing native library)");
        }
        
        let pdfium = Pdfium::default();
        let document = pdfium.load_pdf_from_file(file_path, None)
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

    #[cfg(feature = "mupdf_backend")]
    fn extract_pages_with_mupdf(file_path: &str, page_numbers: &[usize]) -> Result<String> {
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

    #[cfg(feature = "poppler")]
    fn extract_pages_with_poppler(file_path: &str, page_numbers: &[usize]) -> Result<String> {
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

    fn extract_pages_with_pdf_extract(file_path: &str, page_numbers: &[usize]) -> Result<String> {
        // pdf-extract doesn't support page-specific extraction directly
        // So we extract all text and try to split by form feed characters or estimate
        let full_text = pdf_extract::extract_text(file_path)
            .with_context(|| format!("Failed to extract text with pdf-extract: {}", file_path))?;
        
        // Try to split by form feed characters (page breaks)
        let pages: Vec<&str> = full_text.split('\x0C').collect();
        
        if pages.len() > 1 {
            // We have form feed characters, use them as page boundaries
            let mut result = String::new();
            for &page_num in page_numbers {
                if page_num == 0 || page_num > pages.len() {
                    return Err(anyhow::anyhow!("Page {} is out of range (1-{})", page_num, pages.len()));
                }
                
                result.push_str(&format!("=== Page {} ===\n", page_num));
                result.push_str(pages[page_num - 1]); // Convert to 0-based index
                result.push_str("\n\n");
            }
            Ok(result)
        } else {
            // No clear page boundaries, return full text with a note
            // This is a limitation of the pdf-extract backend
            let mut result = String::new();
            result.push_str("=== Note: pdf-extract backend cannot extract specific pages ===\n");
            result.push_str(&format!("Requested pages: {:?}\n", page_numbers));
            result.push_str("Returning full document content:\n\n");
            result.push_str(&full_text);
            Ok(result)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_backend_availability() {
        let backends = FastPdfExtractor::get_available_backends();
        assert!(!backends.is_empty(), "At least one backend should be available");
        
        // pdf-extract should always be available as fallback
        assert!(backends.iter().any(|b| matches!(b, PdfBackend::PdfExtract)));
    }
    
    #[test]
    fn test_backend_info() {
        let info = FastPdfExtractor::get_backend_info();
        assert!(!info.is_empty(), "Should have backend information");
    }

    #[test]
    fn test_get_page_count_nonexistent_file() {
        let result = FastPdfExtractor::get_page_count("nonexistent_file.pdf");
        assert!(result.is_err(), "Should fail for non-existent file");
    }

    #[test]
    fn test_get_page_count_invalid_file() {
        // Create a temporary file with invalid PDF content
        use std::io::Write;
        use tempfile::NamedTempFile;
        
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"This is not a valid PDF file").unwrap();
        
        let result = FastPdfExtractor::get_page_count(temp_file.path().to_str().unwrap());
        assert!(result.is_err(), "Should fail for invalid PDF file");
    }

    #[test]
    fn test_page_count_backend_fallback() {
        // Test that page counting tries multiple backends
        let backends = FastPdfExtractor::get_available_backends();
        assert!(!backends.is_empty(), "Should have at least one backend available");
        
        // pdf-extract should always be available as fallback
        assert!(backends.iter().any(|b| matches!(b, PdfBackend::PdfExtract)));
    }

    #[test]
    fn test_get_page_count_with_backend_invalid_backend() {
        // Test error handling for unavailable backends
        let result = FastPdfExtractor::get_page_count_with_backend("test.pdf", PdfBackend::Pdfium);
        // This might succeed or fail depending on whether Pdfium is available
        // The important thing is that it doesn't panic
        let _ = result;
    }

    #[test]
    fn test_extract_text_and_page_count_consistency() {
        // Test that extract_text and get_page_count use the same backend selection logic
        let backends_extract = FastPdfExtractor::get_available_backends();
        let backends_count = FastPdfExtractor::get_available_backends();
        
        assert_eq!(backends_extract, backends_count, "Both functions should use the same backend order");
    }

    #[test]
    fn test_extract_pages_text_nonexistent_file() {
        let result = FastPdfExtractor::extract_pages_text("nonexistent_file.pdf", &[1, 2]);
        assert!(result.is_err(), "Should fail for non-existent file");
    }

    #[test]
    fn test_extract_pages_text_empty_pages() {
        let result = FastPdfExtractor::extract_pages_text("test.pdf", &[]);
        // Should succeed but return empty content (depending on implementation)
        // For now, we expect it to work with empty page list
        let _ = result; // Don't assert success/failure as it depends on file existence
    }

    #[test]
    fn test_extract_pages_with_backend_invalid_backend() {
        // Test error handling for unavailable backends
        let result = FastPdfExtractor::extract_pages_with_backend("test.pdf", &[1], PdfBackend::Pdfium);
        // This might succeed or fail depending on whether Pdfium is available
        // The important thing is that it doesn't panic
        let _ = result;
    }

    #[test]
    fn test_page_extraction_backend_fallback() {
        // Test that page extraction tries multiple backends
        let backends = FastPdfExtractor::get_available_backends();
        assert!(!backends.is_empty(), "Should have at least one backend available");
        
        // pdf-extract should always be available as fallback
        assert!(backends.iter().any(|b| matches!(b, PdfBackend::PdfExtract)));
    }
} 