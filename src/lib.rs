/// Office Reader MCP - A Model Context Protocol server for reading office documents
/// Supports Excel, PDF, DOCX, and PowerPoint files with streaming capabilities

pub mod document_parser;
pub mod mcp_handler;
pub mod streaming_parser;
pub mod fast_pdf_extractor;
pub mod shared_utils;
pub mod powerpoint_parser;
pub mod cache_system;

/// Re-export the OfficeReader for direct usage
pub use mcp_handler::OfficeReader;

/// Re-export main functionality
pub use document_parser::{
    DocumentProcessingResult, 
    DocumentPageInfoResult,
    ExcelCache,
    DocxCache,
    process_document_with_pages, 
    get_document_page_info,
    read_excel_to_markdown,
    read_docx_to_markdown
};

/// Re-export PowerPoint functionality
pub use powerpoint_parser::{
    PowerPointProcessingResult,
    PowerPointPageInfoResult,
    PowerPointCache,
    SlideSnapshotResult,
    process_powerpoint_with_slides,
    get_powerpoint_slide_info,
    generate_slide_snapshot,
    extract_powerpoint_text_manual,
    get_powerpoint_slide_count,
};

/// Re-export streaming functionality
pub use streaming_parser::{
    ProcessingProgress, 
    StreamingConfig, 
    stream_pdf_to_markdown, 
    stream_excel_to_markdown
};

/// Re-export shared utilities
pub use shared_utils::{
    PdfCache,
    parse_pages_parameter,
    get_or_cache_pdf_content,
    extract_pages_from_cache,
    extract_char_range_from_cache,
    clear_pdf_cache,
    clear_excel_cache,
    clear_docx_cache,
    clear_powerpoint_cache,
    clear_all_caches,
    get_cache_stats,
    get_all_cache_stats,
    validate_file_path,
    generate_file_header,
    generate_chunk_header,
    break_at_word_boundary
};

/// Re-export fast PDF extraction
pub use fast_pdf_extractor::{FastPdfExtractor, PdfBackend};

/// Re-export caching system
pub use cache_system::{CacheableContent, CacheEntry}; 