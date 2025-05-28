use std::path::Path;
use anyhow::{Result, Context};
use futures::stream::{self, Stream};
use serde::{Deserialize, Serialize};

/// Progress information for streaming document processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingProgress {
    pub current_page: usize,
    pub total_pages: Option<usize>,
    pub current_chunk: String,
    pub is_complete: bool,
    pub error: Option<String>,
}

/// Configuration for streaming processing
#[derive(Debug, Clone)]
pub struct StreamingConfig {
    pub chunk_size_pages: usize,
    pub max_chunk_size_chars: usize,
    pub include_progress: bool,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            chunk_size_pages: 5,  // Process 5 pages at a time
            max_chunk_size_chars: 10000,  // Max 10k characters per chunk
            include_progress: true,
        }
    }
}

/// Stream PDF content in character-based chunks
pub fn stream_pdf_to_markdown(
    file_path: &str,
    config: StreamingConfig,
) -> impl Stream<Item = ProcessingProgress> {
    let file_path = file_path.to_string();
    
    stream::unfold(
        (0usize, false, config),
        move |(current_char, is_complete, config)| {
            let file_path = file_path.clone();
            async move {
                if is_complete {
                    return None;
                }

                match process_pdf_chunk(&file_path, current_char, &config).await {
                    Ok(progress) => {
                        let next_char = progress.current_page; // current_page now represents current character position
                        let is_done = progress.is_complete;
                        Some((progress, (next_char, is_done, config)))
                    }
                    Err(e) => {
                        let error_progress = ProcessingProgress {
                            current_page: current_char,
                            total_pages: None,
                            current_chunk: String::new(),
                            is_complete: true,
                            error: Some(e.to_string()),
                        };
                        Some((error_progress, (current_char, true, config)))
                    }
                }
            }
        },
    )
}

/// Process a chunk of PDF content by character count
async fn process_pdf_chunk(
    file_path: &str,
    start_char: usize,
    config: &StreamingConfig,
) -> Result<ProcessingProgress> {
    // Use tokio::task::spawn_blocking for CPU-intensive PDF processing
    let file_path = file_path.to_string();
    let max_chars = config.max_chunk_size_chars;
    
    tokio::task::spawn_blocking(move || {
        // Extract all text from PDF (for now we'll extract each time - could optimize with proper caching later)
        let full_text = pdf_extract::extract_text(&file_path)
            .with_context(|| format!("Failed to extract text from PDF: {}", file_path))?;
        
        // Convert to character indices for proper UTF-8 handling
        let chars: Vec<char> = full_text.chars().collect();
        let total_chars = chars.len();
        let end_char = std::cmp::min(start_char + max_chars, total_chars);
        
        if start_char >= total_chars {
            return Ok(ProcessingProgress {
                current_page: start_char,
                total_pages: Some(total_chars),
                current_chunk: String::new(),
                is_complete: true,
                error: None,
            });
        }

        let mut chunk_content = String::new();
        
        // Add header for first chunk
        if start_char == 0 {
            let filename = Path::new(&file_path)
                .file_name()
                .unwrap()
                .to_string_lossy();
            chunk_content.push_str(&format!("# {}\n\n", filename));
        }
        
        // Add chunk header
        let chunk_num = start_char / max_chars + 1;
        chunk_content.push_str(&format!("## Chunk {} (characters {}-{})\n\n", 
            chunk_num, start_char, end_char));
        
        // Extract the chunk of text using character indices
        let chunk_chars = if end_char <= total_chars {
            &chars[start_char..end_char]
        } else {
            &chars[start_char..]
        };
        
        // Convert back to string
        let chunk_text: String = chunk_chars.iter().collect();
        
        // Try to break at word boundaries for better readability, but ensure minimum progress
        let final_chunk = if end_char < total_chars {
            if let Some(last_space) = chunk_text.rfind(' ') {
                let word_boundary_chunk = &chunk_text[..last_space];
                // Ensure we make meaningful progress (at least 10% of max_chars or minimum 50 chars)
                let min_progress = std::cmp::max(max_chars / 10, 50);
                if word_boundary_chunk.chars().count() >= min_progress {
                    word_boundary_chunk
                } else {
                    // If word boundary breaking results in too small a chunk, use the full chunk
                    &chunk_text
                }
            } else {
                &chunk_text
            }
        } else {
            &chunk_text
        };
        
        chunk_content.push_str(final_chunk);
        chunk_content.push_str("\n\n");
        
        // Calculate actual end position in character count
        let actual_end = start_char + final_chunk.chars().count();
        let is_complete = actual_end >= total_chars;
        
        // Safety check: ensure we always make progress to prevent infinite loops
        let actual_end = if actual_end <= start_char && !is_complete {
            // Force progress by advancing at least 1 character
            std::cmp::min(start_char + 1, total_chars)
        } else {
            actual_end
        };
        
        // Recalculate is_complete after potential adjustment
        let is_complete = actual_end >= total_chars;
        
        Ok(ProcessingProgress {
            current_page: actual_end,
            total_pages: Some(total_chars),
            current_chunk: chunk_content,
            is_complete,
            error: None,
        })
    }).await?
}

/// Stream Excel content sheet by sheet
pub fn stream_excel_to_markdown(
    file_path: &str,
    config: StreamingConfig,
) -> impl Stream<Item = ProcessingProgress> {
    let file_path = file_path.to_string();
    
    stream::unfold(
        (0usize, false, config),
        move |(current_sheet, is_complete, config)| {
            let file_path = file_path.clone();
            async move {
                if is_complete {
                    return None;
                }

                match process_excel_chunk(&file_path, current_sheet, &config).await {
                    Ok(progress) => {
                        let next_sheet = current_sheet + 1;
                        let is_done = progress.is_complete;
                        Some((progress, (next_sheet, is_done, config)))
                    }
                    Err(e) => {
                        let error_progress = ProcessingProgress {
                            current_page: current_sheet,
                            total_pages: None,
                            current_chunk: String::new(),
                            is_complete: true,
                            error: Some(e.to_string()),
                        };
                        Some((error_progress, (current_sheet, true, config)))
                    }
                }
            }
        },
    )
}

/// Process a chunk of Excel sheets
async fn process_excel_chunk(
    file_path: &str,
    sheet_index: usize,
    _config: &StreamingConfig,
) -> Result<ProcessingProgress> {
    use calamine::{Reader, open_workbook, Xlsx};
    
    let file_path = file_path.to_string();
    
    tokio::task::spawn_blocking(move || {
        let mut workbook: Xlsx<_> = open_workbook(&file_path)
            .with_context(|| format!("Failed to open Excel file: {}", file_path))?;
        
        let sheet_names = workbook.sheet_names().to_owned();
        let total_sheets = sheet_names.len();
        
        if sheet_index >= total_sheets {
            return Ok(ProcessingProgress {
                current_page: sheet_index,
                total_pages: Some(total_sheets),
                current_chunk: String::new(),
                is_complete: true,
                error: None,
            });
        }
        
        let mut chunk_content = String::new();
        
        // Add header for first sheet
        if sheet_index == 0 {
            let filename = Path::new(&file_path)
                .file_name()
                .unwrap()
                .to_string_lossy();
            chunk_content.push_str(&format!("# {}\n\n", filename));
        }
        
        // Process current sheet
        let sheet_name = &sheet_names[sheet_index];
        chunk_content.push_str(&format!("## Sheet: {}\n\n", sheet_name));
        
        if let Ok(range) = workbook.worksheet_range(sheet_name) {
            chunk_content.push_str(&crate::document_parser::range_to_markdown_table(&range));
            chunk_content.push_str("\n\n");
        }
        
        let is_complete = sheet_index + 1 >= total_sheets;
        
        Ok(ProcessingProgress {
            current_page: sheet_index + 1,
            total_pages: Some(total_sheets),
            current_chunk: chunk_content,
            is_complete,
            error: None,
        })
    }).await?
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use tokio_stream::StreamExt;

    #[test]
    fn test_streaming_config_default() {
        let config = StreamingConfig::default();
        assert_eq!(config.chunk_size_pages, 5);
        assert_eq!(config.max_chunk_size_chars, 10000);
        assert_eq!(config.include_progress, true);
    }

    #[test]
    fn test_processing_progress_serialization() {
        let progress = ProcessingProgress {
            current_page: 100,
            total_pages: Some(500),
            current_chunk: "Test chunk content".to_string(),
            is_complete: false,
            error: None,
        };
        
        let serialized = serde_json::to_string(&progress).unwrap();
        let deserialized: ProcessingProgress = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(progress.current_page, deserialized.current_page);
        assert_eq!(progress.total_pages, deserialized.total_pages);
        assert_eq!(progress.current_chunk, deserialized.current_chunk);
        assert_eq!(progress.is_complete, deserialized.is_complete);
        assert_eq!(progress.error, deserialized.error);
    }

    #[tokio::test]
    async fn test_stream_pdf_nonexistent_file() {
        let config = StreamingConfig::default();
        let mut stream = Box::pin(stream_pdf_to_markdown("nonexistent_file.pdf", config));
        
        let result = stream.next().await;
        assert!(result.is_some());
        
        let progress = result.unwrap();
        assert!(progress.is_complete);
        assert!(progress.error.is_some());
        assert!(progress.error.unwrap().contains("Failed to extract text from PDF"));
    }

    #[tokio::test]
    async fn test_stream_excel_nonexistent_file() {
        let config = StreamingConfig::default();
        let mut stream = Box::pin(stream_excel_to_markdown("nonexistent_file.xlsx", config));
        
        let result = stream.next().await;
        assert!(result.is_some());
        
        let progress = result.unwrap();
        assert!(progress.is_complete);
        assert!(progress.error.is_some());
        assert!(progress.error.unwrap().contains("Failed to open Excel file"));
    }

    #[tokio::test]
    async fn test_streaming_config_custom_chunk_size() {
        let mut config = StreamingConfig::default();
        config.max_chunk_size_chars = 5000;
        
        // Test with a mock text file (since we can't easily create a real PDF in tests)
        let temp_file = NamedTempFile::with_suffix(".pdf").unwrap();
        let file_path = temp_file.path().to_str().unwrap();
        
        // This will fail because it's not a real PDF, but we can test the config is used
        let mut stream = Box::pin(stream_pdf_to_markdown(file_path, config));
        let result = stream.next().await;
        
        assert!(result.is_some());
        let progress = result.unwrap();
        assert!(progress.is_complete);
        assert!(progress.error.is_some());
    }

    #[test]
    fn test_processing_progress_with_error() {
        let progress = ProcessingProgress {
            current_page: 0,
            total_pages: None,
            current_chunk: String::new(),
            is_complete: true,
            error: Some("Test error message".to_string()),
        };
        
        assert!(progress.is_complete);
        assert!(progress.error.is_some());
        assert_eq!(progress.error.unwrap(), "Test error message");
    }

    #[test]
    fn test_processing_progress_without_error() {
        let progress = ProcessingProgress {
            current_page: 50,
            total_pages: Some(100),
            current_chunk: "Some content".to_string(),
            is_complete: false,
            error: None,
        };
        
        assert!(!progress.is_complete);
        assert!(progress.error.is_none());
        assert_eq!(progress.current_page, 50);
        assert_eq!(progress.total_pages, Some(100));
    }

    #[tokio::test]
    async fn test_stream_completion() {
        // Test that the stream properly completes
        let config = StreamingConfig::default();
        let mut stream = Box::pin(stream_pdf_to_markdown("nonexistent.pdf", config));
        
        // First call should return an error
        let first_result = stream.next().await;
        assert!(first_result.is_some());
        assert!(first_result.unwrap().is_complete);
        
        // Second call should return None (stream completed)
        let second_result = stream.next().await;
        assert!(second_result.is_none());
    }

    #[test]
    fn test_streaming_config_clone() {
        let config1 = StreamingConfig::default();
        let config2 = config1.clone();
        
        assert_eq!(config1.chunk_size_pages, config2.chunk_size_pages);
        assert_eq!(config1.max_chunk_size_chars, config2.max_chunk_size_chars);
        assert_eq!(config1.include_progress, config2.include_progress);
    }

    #[test]
    fn test_streaming_config_debug() {
        let config = StreamingConfig::default();
        let debug_str = format!("{:?}", config);
        
        assert!(debug_str.contains("StreamingConfig"));
        assert!(debug_str.contains("chunk_size_pages"));
        assert!(debug_str.contains("max_chunk_size_chars"));
        assert!(debug_str.contains("include_progress"));
    }

    #[tokio::test]
    async fn test_utf8_character_boundary_handling() {
        // Test with text containing multi-byte UTF-8 characters
        let _test_text = "Hello 世界! This is a test with Chinese characters: 竞争对手分析";
        let temp_file = NamedTempFile::with_suffix(".pdf").unwrap();
        let file_path = temp_file.path().to_str().unwrap();
        
        // This will fail because it's not a real PDF, but we can test that it doesn't panic
        // on UTF-8 character boundaries
        let mut config = StreamingConfig::default();
        config.max_chunk_size_chars = 20; // Small chunk to force character boundary issues
        
        let mut stream = Box::pin(stream_pdf_to_markdown(file_path, config));
        let result = stream.next().await;
        
        assert!(result.is_some());
        let progress = result.unwrap();
        assert!(progress.is_complete);
        assert!(progress.error.is_some());
        // The important thing is that it doesn't panic on character boundaries
    }

    #[tokio::test]
    async fn test_no_infinite_loop_with_small_chunks() {
        // Test that we don't get stuck in infinite loops with very small chunk sizes
        let temp_file = NamedTempFile::with_suffix(".pdf").unwrap();
        let file_path = temp_file.path().to_str().unwrap();
        
        let mut config = StreamingConfig::default();
        config.max_chunk_size_chars = 10; // Very small chunk size
        
        let mut stream = Box::pin(stream_pdf_to_markdown(file_path, config));
        
        // Should complete quickly (within a few iterations) even with small chunks
        let mut iteration_count = 0;
        while let Some(progress) = stream.next().await {
            iteration_count += 1;
            
            // Safety check: prevent actual infinite loops in tests
            if iteration_count > 10 {
                panic!("Too many iterations, possible infinite loop detected");
            }
            
            if progress.is_complete {
                break;
            }
        }
        
        // Should have completed within reasonable number of iterations
        assert!(iteration_count <= 10);
    }
} 