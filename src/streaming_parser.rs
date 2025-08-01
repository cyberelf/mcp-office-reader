use std::path::Path;
use anyhow::{Result, Context};
use futures::stream::{self, Stream};
use serde::{Deserialize, Serialize};
use crate::shared_utils::{
    get_or_cache_pdf_content, extract_char_range_from_cache,
    generate_file_header, generate_chunk_header,
    break_at_word_boundary
};

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
    pub max_chunk_size_chars: usize,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            max_chunk_size_chars: 10000,  // Max 10k characters per chunk
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

/// Process a chunk of PDF content by character count (optimized version)
async fn process_pdf_chunk(
    file_path: &str,
    start_char: usize,
    config: &StreamingConfig,
) -> Result<ProcessingProgress> {
    // Use tokio::task::spawn_blocking for CPU-intensive PDF processing
    let file_path = file_path.to_string();
    let max_chars = config.max_chunk_size_chars;
    
    tokio::task::spawn_blocking(move || {
        // Get cached PDF content (much faster than re-extracting)
        let pdf_cache = get_or_cache_pdf_content(&file_path)?;
        let total_chars = pdf_cache.char_indices.len().saturating_sub(1);
        
        if start_char >= total_chars {
            return Ok(ProcessingProgress {
                current_page: start_char,
                total_pages: Some(total_chars),
                current_chunk: String::new(),
                is_complete: true,
                error: None,
            });
        }

        let end_char = std::cmp::min(start_char + max_chars, total_chars);
        let mut chunk_content = String::new();
        
        // Add header for first chunk
        if start_char == 0 {
            chunk_content.push_str(&generate_file_header(&file_path));
        }
        
        // Add chunk header
        let chunk_num = start_char / max_chars + 1;
        chunk_content.push_str(&generate_chunk_header(chunk_num, start_char, end_char, "characters"));
        
        // Extract the chunk using shared utility
        let chunk_text = extract_char_range_from_cache(&pdf_cache, start_char, end_char)?;
        
        // Try to break at word boundaries for better readability
        let final_chunk = break_at_word_boundary(&chunk_text, max_chars);
        
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