/// Office Reader MCP - A Model Context Protocol server for reading office documents
/// Supports Excel, PDF, and DOCX files with streaming capabilities

pub mod document_parser;
pub mod mcp_handler;
pub mod streaming_parser;
pub mod fast_pdf_extractor;

/// Re-export the OfficeReader for direct usage
pub use mcp_handler::OfficeReader;

/// Re-export main functionality
pub use document_parser::{
    DocumentProcessingResult, 
    process_document_with_pages, 
    get_document_page_info,
    read_excel_to_markdown,
    read_docx_to_markdown
};

/// Re-export streaming functionality
pub use streaming_parser::{
    ProcessingProgress, 
    StreamingConfig, 
    stream_pdf_to_markdown, 
    stream_excel_to_markdown,
    clear_pdf_cache,
    get_cache_stats
};

/// Re-export fast PDF extraction
pub use fast_pdf_extractor::{FastPdfExtractor, PdfBackend}; 