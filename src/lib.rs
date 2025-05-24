pub mod document_parser;
pub mod mcp_handler;

/// Re-export the main process_document function for convenience
pub use document_parser::process_document;

/// Re-export the OfficeReader for direct usage
pub use mcp_handler::OfficeReader; 