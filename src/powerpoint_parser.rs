use std::path::Path;
use std::fs::File;
use std::io::Read;
use std::collections::HashMap;

use anyhow::{Result, Context};
use zip::ZipArchive;
use quick_xml::Reader;
use quick_xml::events::Event;

/// Result of PowerPoint processing with slide-based support
#[derive(Debug, Clone)]
pub struct PowerPointProcessingResult {
    pub content: String,
    pub total_slides: Option<usize>,
    pub requested_slides: String,
    pub returned_slides: Vec<usize>,
    pub file_path: String,
    pub slide_texts: HashMap<usize, String>,
    pub error: Option<String>,
}

/// PowerPoint slide snapshot result
#[derive(Debug, Clone)]
pub struct SlideSnapshotResult {
    pub slide_number: usize,
    pub image_data: Option<Vec<u8>>,
    pub image_format: String,
    pub error: Option<String>,
}

/// PowerPoint page information result
#[derive(Debug, Clone)]
pub struct PowerPointPageInfoResult {
    pub file_path: String,
    pub total_slides: Option<usize>,
    pub slide_info: String,
    pub error: Option<String>,
}

impl PowerPointProcessingResult {
    /// Create a new result for successful processing
    pub fn success(
        content: String,
        total_slides: Option<usize>,
        requested_slides: String,
        returned_slides: Vec<usize>,
        file_path: String,
        slide_texts: HashMap<usize, String>,
    ) -> Self {
        Self {
            content,
            total_slides,
            requested_slides,
            returned_slides,
            file_path,
            slide_texts,
            error: None,
        }
    }

    /// Create a new result for error cases
    pub fn error(file_path: String, error: String) -> Self {
        Self {
            content: error.clone(),
            total_slides: None,
            requested_slides: String::new(),
            returned_slides: Vec::new(),
            file_path,
            slide_texts: HashMap::new(),
            error: Some(error),
        }
    }
}

impl PowerPointPageInfoResult {
    /// Create a new result for successful page info retrieval
    pub fn success(
        file_path: String,
        total_slides: Option<usize>,
        slide_info: String,
    ) -> Self {
        Self {
            file_path,
            total_slides,
            slide_info,
            error: None,
        }
    }

    /// Create a new result for error cases
    pub fn error(file_path: String, error: String) -> Self {
        Self {
            file_path,
            total_slides: None,
            slide_info: String::new(),
            error: Some(error),
        }
    }

    /// Check if the file exists (no error or error is not file_not_found)
    pub fn file_exists(&self) -> bool {
        self.error.as_ref() != Some(&"file_not_found".to_string())
    }
}

impl SlideSnapshotResult {
    /// Create a new result for successful snapshot
    pub fn success(
        slide_number: usize,
        image_data: Vec<u8>,
        image_format: String,
    ) -> Self {
        Self {
            slide_number,
            image_data: Some(image_data),
            image_format,
            error: None,
        }
    }

    /// Create a new result for error cases
    pub fn error(slide_number: usize, error: String) -> Self {
        Self {
            slide_number,
            image_data: None,
            image_format: String::new(),
            error: Some(error),
        }
    }
}

/// Extract text from PowerPoint file by manually parsing PPTX structure
pub fn extract_powerpoint_text_manual(file_path: &str) -> Result<(String, HashMap<usize, String>)> {
    let file = File::open(file_path)
        .with_context(|| format!("Failed to open PowerPoint file: {}", file_path))?;
    
    let mut archive = ZipArchive::new(file)
        .with_context(|| "Failed to read PowerPoint file as ZIP archive")?;
    
    let mut slide_texts = HashMap::new();
    let mut all_text = String::new();
    
    // Find all slide files
    let slide_files: Vec<String> = (0..archive.len())
        .filter_map(|i| {
            if let Ok(file) = archive.by_index(i) {
                let name = file.name();
                if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") {
                    Some(name.to_string())
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();
    
    // Sort slide files to ensure proper order
    let mut sorted_slides = slide_files;
    sorted_slides.sort_by(|a, b| {
        let a_num = extract_slide_number(a);
        let b_num = extract_slide_number(b);
        a_num.cmp(&b_num)
    });
    
    // Extract text from each slide
    for (index, slide_file) in sorted_slides.iter().enumerate() {
        let slide_number = index + 1;
        
        if let Ok(mut file) = archive.by_name(slide_file) {
            let mut contents = String::new();
            if file.read_to_string(&mut contents).is_ok() {
                let slide_text = extract_text_from_slide_xml(&contents)?;
                slide_texts.insert(slide_number, slide_text.clone());
                
                if !slide_text.trim().is_empty() {
                    all_text.push_str(&format!("## Slide {}\n\n{}\n\n", slide_number, slide_text));
                }
            }
        }
    }
    
    Ok((all_text, slide_texts))
}

/// Extract slide number from slide file name
fn extract_slide_number(filename: &str) -> usize {
    // Extract number from "ppt/slides/slide1.xml" format
    if let Some(start) = filename.rfind("slide") {
        if let Some(end) = filename.rfind(".xml") {
            let number_str = &filename[start + 5..end];
            return number_str.parse().unwrap_or(0);
        }
    }
    0
}

/// Extract text content from slide XML
fn extract_text_from_slide_xml(xml_content: &str) -> Result<String> {
    let mut reader = Reader::from_str(xml_content);
    reader.config_mut().trim_text(true);
    
    let mut text_content = String::new();
    let mut in_text_element = false;
    let mut buf = Vec::new();
    
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                match e.name().as_ref() {
                    b"a:t" => in_text_element = true,
                    _ => {}
                }
            }
            Ok(Event::End(ref e)) => {
                match e.name().as_ref() {
                    b"a:t" => in_text_element = false,
                    _ => {}
                }
            }
            Ok(Event::Text(e)) => {
                if in_text_element {
                    let text = e.unescape().unwrap_or_default();
                    text_content.push_str(&text);
                    text_content.push(' ');
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                log::warn!("Error parsing slide XML: {}", e);
                break;
            }
            _ => {}
        }
        buf.clear();
    }
    
    Ok(text_content.trim().to_string())
}

/// Get PowerPoint slide count
pub fn get_powerpoint_slide_count(file_path: &str) -> Result<usize> {
    let file = File::open(file_path)
        .with_context(|| format!("Failed to open PowerPoint file: {}", file_path))?;
    
    let mut archive = ZipArchive::new(file)
        .with_context(|| "Failed to read PowerPoint file as ZIP archive")?;
    
    // Count slide files
    let slide_count = (0..archive.len())
        .filter(|&i| {
            if let Ok(file) = archive.by_index(i) {
                let name = file.name();
                name.starts_with("ppt/slides/slide") && name.ends_with(".xml")
            } else {
                false
            }
        })
        .count();
    
    Ok(slide_count)
}

/// Convert PowerPoint to markdown with slide-based selection
pub fn process_powerpoint_with_slides(
    file_path: &str,
    slides: Option<String>,
) -> PowerPointProcessingResult {
    use crate::shared_utils::{parse_pages_parameter, validate_file_path};
    
    let file_path_string = file_path.to_string();
    let slides = slides.unwrap_or_else(|| "all".to_string());
    
    // Validate file
    if let Err(e) = validate_file_path(file_path) {
        return PowerPointProcessingResult::error(file_path_string, e);
    }
    
    // Get slide count
    let total_slides = match get_powerpoint_slide_count(file_path) {
        Ok(count) => count,
        Err(e) => return PowerPointProcessingResult::error(
            file_path_string,
            format!("Failed to get slide count: {}", e),
        ),
    };
    
    // Parse the slides parameter
    let requested_slide_indices = match parse_pages_parameter(&slides, total_slides) {
        Ok(indices) => indices,
        Err(e) => return PowerPointProcessingResult::error(
            file_path_string,
            format!("Invalid slides parameter: {}", e),
        ),
    };
    
    // Extract text from slides using manual parsing
    let (all_text, slide_texts) = match extract_powerpoint_text_manual(file_path) {
        Ok((text, slides)) => (text, slides),
        Err(e) => return PowerPointProcessingResult::error(
            file_path_string,
            format!("Failed to extract PowerPoint text: {}", e),
        ),
    };
    
    // Build markdown content for requested slides
    let mut markdown = format!("# {}\n\n", Path::new(file_path).file_name().unwrap().to_string_lossy());
    
    if requested_slide_indices.len() == total_slides {
        // All slides requested
        markdown.push_str("## Content (All Slides)\n\n");
        markdown.push_str(&all_text);
    } else {
        // Specific slides requested
        markdown.push_str(&format!("## Content (Slides: {})\n\n", slides));
        
        for &slide_index in &requested_slide_indices {
            if let Some(slide_text) = slide_texts.get(&slide_index) {
                markdown.push_str(&format!("### Slide {}\n\n{}\n\n", slide_index, slide_text));
            } else {
                markdown.push_str(&format!("### Slide {}\n\n*No text content found*\n\n", slide_index));
            }
        }
    }
    
    PowerPointProcessingResult::success(
        markdown,
        Some(total_slides),
        slides,
        requested_slide_indices,
        file_path_string,
        slide_texts,
    )
}

/// Get PowerPoint slide information
pub fn get_powerpoint_slide_info(file_path: &str) -> PowerPointPageInfoResult {
    use crate::shared_utils::validate_file_path;
    
    let file_path_string = file_path.to_string();
    
    // Validate file
    if let Err(e) = validate_file_path(file_path) {
        if e.contains("File not found") {
            return PowerPointPageInfoResult::error(file_path_string, "file_not_found".to_string());
        } else {
            return PowerPointPageInfoResult::error(file_path_string, e);
        }
    }
    
    // Get slide count
    match get_powerpoint_slide_count(file_path) {
        Ok(slide_count) => {
            PowerPointPageInfoResult::success(
                file_path_string,
                Some(slide_count),
                format!("PowerPoint file with {} slides", slide_count),
            )
        },
        Err(e) => PowerPointPageInfoResult::error(
            file_path_string,
            format!("Failed to analyze PowerPoint file: {}", e),
        ),
    }
}

/// Generate slide snapshot (requires external tools like LibreOffice)
pub fn generate_slide_snapshot(
    _file_path: &str,
    slide_number: usize,
    _output_format: &str, // "png", "jpg", etc.
) -> SlideSnapshotResult {
    // This is a placeholder implementation
    // In a real implementation, you would:
    // 1. Use LibreOffice/unoconv to convert specific slide to image
    // 2. Or convert entire presentation to PDF and then extract specific page as image
    // 3. Or use a PowerPoint-specific rendering library
    
    SlideSnapshotResult::error(
        slide_number,
        "Slide snapshot generation not yet implemented. Consider using LibreOffice headless mode or PDF conversion.".to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_slide_number() {
        assert_eq!(extract_slide_number("ppt/slides/slide1.xml"), 1);
        assert_eq!(extract_slide_number("ppt/slides/slide10.xml"), 10);
        assert_eq!(extract_slide_number("ppt/slides/slide123.xml"), 123);
        assert_eq!(extract_slide_number("invalid.xml"), 0);
    }

    #[test]
    fn test_powerpoint_processing_result_success() {
        let slide_texts = HashMap::new();
        let result = PowerPointProcessingResult::success(
            "test content".to_string(),
            Some(5),
            "1,3,5".to_string(),
            vec![1, 3, 5],
            "test.pptx".to_string(),
            slide_texts,
        );
        
        assert_eq!(result.content, "test content");
        assert_eq!(result.total_slides, Some(5));
        assert_eq!(result.requested_slides, "1,3,5");
        assert_eq!(result.returned_slides, vec![1, 3, 5]);
        assert_eq!(result.file_path, "test.pptx");
        assert!(result.error.is_none());
    }

    #[test]
    fn test_powerpoint_processing_result_error() {
        let result = PowerPointProcessingResult::error(
            "test.pptx".to_string(),
            "Test error message".to_string(),
        );
        
        assert_eq!(result.content, "Test error message");
        assert_eq!(result.total_slides, None);
        assert_eq!(result.requested_slides, "");
        assert_eq!(result.returned_slides, Vec::<usize>::new());
        assert_eq!(result.file_path, "test.pptx");
        assert_eq!(result.error.as_ref().unwrap(), "Test error message");
    }

    #[test]
    fn test_powerpoint_page_info_result_success() {
        let result = PowerPointPageInfoResult::success(
            "test.pptx".to_string(),
            Some(5),
            "PowerPoint file with 5 slides".to_string(),
        );
        
        assert_eq!(result.file_path, "test.pptx");
        assert_eq!(result.total_slides, Some(5));
        assert_eq!(result.slide_info, "PowerPoint file with 5 slides");
        assert!(result.error.is_none());
        assert!(result.file_exists());
    }

    #[test]
    fn test_powerpoint_page_info_result_file_not_found() {
        let result = PowerPointPageInfoResult::error(
            "nonexistent.pptx".to_string(),
            "file_not_found".to_string(),
        );
        
        assert_eq!(result.file_path, "nonexistent.pptx");
        assert_eq!(result.total_slides, None);
        assert_eq!(result.slide_info, "");
        assert_eq!(result.error.as_ref().unwrap(), "file_not_found");
        assert!(!result.file_exists());
    }

    #[test]
    fn test_slide_snapshot_result_success() {
        let image_data = vec![1, 2, 3, 4];
        let result = SlideSnapshotResult::success(
            1,
            image_data.clone(),
            "png".to_string(),
        );
        
        assert_eq!(result.slide_number, 1);
        assert_eq!(result.image_data, Some(image_data));
        assert_eq!(result.image_format, "png");
        assert!(result.error.is_none());
    }

    #[test]
    fn test_slide_snapshot_result_error() {
        let result = SlideSnapshotResult::error(
            1,
            "Test error".to_string(),
        );
        
        assert_eq!(result.slide_number, 1);
        assert_eq!(result.image_data, None);
        assert_eq!(result.image_format, "");
        assert_eq!(result.error.as_ref().unwrap(), "Test error");
    }

    #[test]
    fn test_get_powerpoint_slide_count_nonexistent_file() {
        let result = get_powerpoint_slide_count("nonexistent.pptx");
        assert!(result.is_err());
    }

    #[test]
    fn test_process_powerpoint_with_slides_nonexistent_file() {
        let result = process_powerpoint_with_slides("nonexistent.pptx", Some("1".to_string()));
        
        assert!(result.error.is_some());
        assert!(result.content.contains("File not found") || 
                result.content.contains("Failed to get slide count"));
    }

    #[test]
    fn test_get_powerpoint_slide_info_nonexistent_file() {
        let result = get_powerpoint_slide_info("nonexistent.pptx");
        
        assert_eq!(result.error.as_ref().unwrap(), "file_not_found");
        assert!(!result.file_exists());
    }

    #[test]
    fn test_extract_text_from_slide_xml() {
        let xml_content = r#"
            <p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main">
                <p:cSld>
                    <p:spTree>
                        <p:sp>
                            <p:txBody>
                                <a:p>
                                    <a:r>
                                        <a:t>Hello World</a:t>
                                    </a:r>
                                </a:p>
                                <a:p>
                                    <a:r>
                                        <a:t>This is a test</a:t>
                                    </a:r>
                                </a:p>
                            </p:txBody>
                        </p:sp>
                    </p:spTree>
                </p:cSld>
            </p:sld>
        "#;
        
        let result = extract_text_from_slide_xml(xml_content);
        assert!(result.is_ok());
        let text = result.unwrap();
        assert!(text.contains("Hello World"));
        assert!(text.contains("This is a test"));
    }
} 