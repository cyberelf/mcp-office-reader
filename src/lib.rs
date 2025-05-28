pub mod document_parser;
pub mod mcp_handler;
pub mod streaming_parser;

/// Re-export the OfficeReader for direct usage
pub use mcp_handler::OfficeReader;

/// Re-export streaming functionality
pub use streaming_parser::{ProcessingProgress, StreamingConfig, stream_pdf_to_markdown, stream_excel_to_markdown}; 