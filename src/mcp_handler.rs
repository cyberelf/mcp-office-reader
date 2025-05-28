use rmcp::{service::RequestContext, RoleServer, Error as McpError, model::ServerInfo, model::{IntoContents, Content}, tool};
use rmcp::model::{Implementation, ListPromptsResult, PaginatedRequestParam, ProtocolVersion, ServerCapabilities};
use rmcp::ServerHandler;
use rmcp::serve_server;
use rmcp::schemars;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use tokio_stream::StreamExt;

use crate::document_parser::{process_document_with_pagination, get_document_text_length, DocumentProcessingResult};
use crate::streaming_parser::{stream_pdf_to_markdown, stream_excel_to_markdown, StreamingConfig, ProcessingProgress};

/// Office document processor struct that implements the MCP tool interface
#[derive(Clone)]
pub struct OfficeReader;

impl OfficeReader {
    pub fn new() -> Self {
        OfficeReader
    }
}

/// Input for the read_office_document tool
#[derive(Serialize, Deserialize, Debug, schemars::JsonSchema)]
pub struct ReadOfficeDocumentInput {
    #[schemars(description = "Path to the office document file")]
    pub file_path: String,
}

/// Input for the stream_office_document tool
#[derive(Serialize, Deserialize, Debug, schemars::JsonSchema)]
pub struct StreamOfficeDocumentInput {
    #[schemars(description = "Path to the office document file")]
    pub file_path: String,
    #[schemars(description = "Maximum characters per chunk (default: 10000)")]
    pub chunk_size: Option<usize>,
}

/// Wrapper for document content to implement IntoContents
pub struct DocumentContent {
    pub content: String,
}

impl IntoContents for DocumentContent {
    fn into_contents(self) -> Vec<Content> {
        vec![Content::text(self.content)]
    }
}

/// Wrapper for document text length information
pub struct DocumentLengthInfo {
    pub file_path: String,
    pub total_length: usize,
    pub file_exists: bool,
    pub error: Option<String>,
}

impl IntoContents for DocumentLengthInfo {
    fn into_contents(self) -> Vec<Content> {
        let info = if self.file_exists {
            if let Some(error) = self.error {
                format!("File: {}\nError: {}", self.file_path, error)
            } else {
                format!("File: {}\nTotal text length: {} characters", self.file_path, self.total_length)
            }
        } else {
            format!("File: {}\nFile not found", self.file_path)
        };
        vec![Content::text(info)]
    }
}

/// Wrapper for partial document content with metadata
pub struct PartialDocumentContent {
    pub content: String,
    pub total_length: usize,
    pub offset: usize,
    pub returned_length: usize,
    pub has_more: bool,
    pub file_path: String,
}

impl IntoContents for PartialDocumentContent {
    fn into_contents(self) -> Vec<Content> {
        let metadata = format!(
            "File: {}\nTotal length: {} characters\nOffset: {}\nReturned: {} characters\nHas more content: {}\n\n",
            self.file_path, self.total_length, self.offset, self.returned_length, self.has_more
        );
        vec![Content::text(format!("{}{}", metadata, self.content))]
    }
}

/// Wrapper for streaming progress to implement IntoContents
pub struct StreamingContent {
    pub progress: ProcessingProgress,
}

impl IntoContents for StreamingContent {
    fn into_contents(self) -> Vec<Content> {
        let progress_json = serde_json::to_string_pretty(&self.progress).unwrap_or_else(|_| "Error serializing progress".to_string());
        vec![Content::text(format!("```json\n{}\n```\n\n{}", progress_json, self.progress.current_chunk))]
    }
}

/// Convert DocumentProcessingResult to PartialDocumentContent
impl From<DocumentProcessingResult> for PartialDocumentContent {
    fn from(result: DocumentProcessingResult) -> Self {
        Self {
            content: result.content,
            total_length: result.total_length,
            offset: result.offset,
            returned_length: result.returned_length,
            has_more: result.has_more,
            file_path: result.file_path,
        }
    }
}

/// Convert DocumentProcessingResult to DocumentLengthInfo
impl From<DocumentProcessingResult> for DocumentLengthInfo {
    fn from(result: DocumentProcessingResult) -> Self {
        let file_exists = result.error.as_ref() != Some(&"file_not_found".to_string());
        let error = if result.error.as_ref() == Some(&"file_not_found".to_string()) {
            None // Don't show error for file not found, just indicate it doesn't exist
        } else {
            result.error
        };
        
        Self {
            file_path: result.file_path,
            total_length: result.total_length,
            file_exists,
            error,
        }
    }
}

#[tool(tool_box)]
impl OfficeReader {
    /// Get the text length of an office document without reading the full content
    #[tool(description = "Get the total text length of an office document (Excel, PDF, DOCX) without reading the full content")]
    pub async fn get_document_text_length(
        &self,
        #[tool(param)]
        #[schemars(description = "Path to the office document file")]
        file_path: String,
    ) -> DocumentLengthInfo {
        let result = get_document_text_length(&file_path);
        result.into()
    }

    /// Read an office document and return its content as markdown with size limits and offset support
    #[tool(description = "Read an office document (Excel, PDF, DOCX) and return its content as markdown with optional size limits and offset")]
    pub async fn read_office_document(
        &self,
        #[tool(param)]
        #[schemars(description = "Path to the office document file")]
        file_path: String,
        #[tool(param)]
        #[schemars(description = "Maximum number of characters to return (default: 50000)")]
        max_size: Option<usize>,
        #[tool(param)]
        #[schemars(description = "Character offset to start reading from (default: 0)")]
        offset: Option<usize>,
    ) -> PartialDocumentContent {
        let result = process_document_with_pagination(&file_path, offset, max_size);
        result.into()
    }

    /// Stream an office document and return its content as markdown in chunks
    #[tool(description = "Stream an office document (Excel, PDF, DOCX) and return its content as markdown in chunks with progress")]
    pub async fn stream_office_document(
        &self,
        #[tool(param)]
        #[schemars(description = "Path to the office document file")]
        file_path: String,
        #[tool(param)]
        #[schemars(description = "Maximum characters per chunk (default: 10000)")]
        chunk_size: Option<usize>,
    ) -> StreamingContent {
        use std::path::Path;
        
        // Create streaming config
        let mut config = StreamingConfig::default();
        if let Some(size) = chunk_size {
            config.max_chunk_size_chars = size;
        }
        
        // Check if file exists
        if !Path::new(&file_path).exists() {
            return StreamingContent {
                progress: ProcessingProgress {
                    current_page: 0,
                    total_pages: None,
                    current_chunk: format!("File not found: {}", file_path),
                    is_complete: true,
                    error: Some(format!("File not found: {}", file_path)),
                }
            };
        }
        
        // Determine file type from extension
        let extension = Path::new(&file_path)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase());
        
        match extension {
            Some(ext) => {
                match ext.as_str() {
                    "pdf" => {
                        // Stream PDF content
                        let mut stream = Box::pin(stream_pdf_to_markdown(&file_path, config));
                        if let Some(progress) = stream.next().await {
                            StreamingContent { progress }
                        } else {
                            StreamingContent {
                                progress: ProcessingProgress {
                                    current_page: 0,
                                    total_pages: None,
                                    current_chunk: "No content found".to_string(),
                                    is_complete: true,
                                    error: Some("No content found".to_string()),
                                }
                            }
                        }
                    }
                    "xlsx" | "xls" => {
                        // Stream Excel content
                        let mut stream = Box::pin(stream_excel_to_markdown(&file_path, config));
                        if let Some(progress) = stream.next().await {
                            StreamingContent { progress }
                        } else {
                            StreamingContent {
                                progress: ProcessingProgress {
                                    current_page: 0,
                                    total_pages: None,
                                    current_chunk: "No content found".to_string(),
                                    is_complete: true,
                                    error: Some("No content found".to_string()),
                                }
                            }
                        }
                    }
                    _ => {
                        StreamingContent {
                            progress: ProcessingProgress {
                                current_page: 0,
                                total_pages: None,
                                current_chunk: format!("Unsupported file type for streaming: {}", ext),
                                is_complete: true,
                                error: Some(format!("Unsupported file type for streaming: {}", ext)),
                            }
                        }
                    }
                }
            }
            None => {
                StreamingContent {
                    progress: ProcessingProgress {
                        current_page: 0,
                        total_pages: None,
                        current_chunk: "Unable to determine file type (no extension)".to_string(),
                        is_complete: true,
                        error: Some("Unable to determine file type (no extension)".to_string()),
                    }
                }
            }
        }
    }
}

#[tool(tool_box)]
impl ServerHandler for OfficeReader {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::default(),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "This server provides functionality to read and parse office documents (Excel, PDF, DOCX) and return their content as markdown. Available tools:\n\n\
                1. get_document_text_length: Get the total text length of a document without reading the full content\n\
                2. read_office_document: Read a document with optional size limits (default: 50,000 chars) and offset support for pagination\n\
                3. read_office_document_legacy: Read a document without size limits (for backward compatibility)\n\
                4. stream_office_document: Stream document content in chunks with progress tracking\n\n\
                For large documents, use get_document_text_length first to check size, then use read_office_document with appropriate offset and max_size parameters to read in chunks.".to_string()
            ),
        }
    }

    async fn list_prompts(
        &self,
        _request: PaginatedRequestParam,
        _: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, McpError> {
        // We don't use prompts in this implementation
        Ok(ListPromptsResult {
            next_cursor: None,
            prompts: vec![],
        })
    }
}

/// Set up the MCP server with our tools
pub async fn start_server() -> Result<()> {
    use tokio::io::{stdin, stdout};
    let transport = (stdin(), stdout());
    
    let office_reader = OfficeReader::new();
    
    // Serve the handler with the transport
    let server = serve_server(office_reader, transport).await?;
    
    let quit_reason = server.waiting().await?;
    println!("Server stopped: {:?}", quit_reason);
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_document_text_length_file_not_found() {
        let office_reader = OfficeReader::new();
        let result = office_reader.get_document_text_length("nonexistent_file.xlsx".to_string()).await;
        
        assert_eq!(result.file_path, "nonexistent_file.xlsx");
        assert_eq!(result.total_length, 0);
        assert!(!result.file_exists);
        assert!(result.error.is_none());
    }

    #[tokio::test]
    async fn test_read_office_document_with_offset_and_size() {
        let office_reader = OfficeReader::new();
        
        // Test with a non-existent file first
        let result = office_reader.read_office_document(
            "nonexistent_file.xlsx".to_string(),
            Some(100),
            Some(0)
        ).await;
        
        assert_eq!(result.file_path, "nonexistent_file.xlsx");
        assert_eq!(result.total_length, 0);
        assert_eq!(result.offset, 0);
        assert_eq!(result.returned_length, result.content.len()); // Length of error message
        assert!(!result.has_more);
        assert!(result.content.contains("File not found"));
    }

    #[tokio::test]
    async fn test_partial_document_content_metadata() {
        let content = PartialDocumentContent {
            content: "Hello, World!".to_string(),
            total_length: 1000,
            offset: 100,
            returned_length: 13,
            has_more: true,
            file_path: "test.txt".to_string(),
        };
        
        // Test the struct fields directly before moving
        assert_eq!(content.total_length, 1000);
        assert_eq!(content.offset, 100);
        assert_eq!(content.returned_length, 13);
        assert!(content.has_more);
        assert_eq!(content.file_path, "test.txt");
        assert_eq!(content.content, "Hello, World!");
        
        let contents = content.into_contents();
        assert_eq!(contents.len(), 1);
    }

    #[tokio::test]
    async fn test_document_length_info_formatting() {
        let length_info = DocumentLengthInfo {
            file_path: "test.pdf".to_string(),
            total_length: 5000,
            file_exists: true,
            error: None,
        };
        
        // Test the struct fields directly before moving
        assert_eq!(length_info.file_path, "test.pdf");
        assert_eq!(length_info.total_length, 5000);
        assert!(length_info.file_exists);
        assert!(length_info.error.is_none());
        
        let contents = length_info.into_contents();
        assert_eq!(contents.len(), 1);
    }

    #[tokio::test]
    async fn test_document_length_info_with_error() {
        let length_info = DocumentLengthInfo {
            file_path: "test.unsupported".to_string(),
            total_length: 0,
            file_exists: true,
            error: Some("Unsupported file type: unsupported".to_string()),
        };
        
        // Test the struct fields directly before moving
        assert_eq!(length_info.file_path, "test.unsupported");
        assert_eq!(length_info.total_length, 0);
        assert!(length_info.file_exists);
        assert!(length_info.error.is_some());
        assert_eq!(length_info.error.as_ref().unwrap(), "Unsupported file type: unsupported");
        
        let contents = length_info.into_contents();
        assert_eq!(contents.len(), 1);
    }

    #[tokio::test]
    async fn test_offset_and_size_limits() {
        // Test the logic for offset and size limits with a mock scenario
        let full_text = "0123456789".repeat(100); // 1000 characters
        let total_length = full_text.len();
        
        // Test normal case
        let max_size = 50;
        let offset = 100;
        let start_pos = offset.min(total_length);
        let end_pos = (start_pos + max_size).min(total_length);
        let content = &full_text[start_pos..end_pos];
        let has_more = end_pos < total_length;
        
        assert_eq!(start_pos, 100);
        assert_eq!(end_pos, 150);
        assert_eq!(content.len(), 50);
        assert!(has_more);
        
        // Test offset beyond content
        let offset = 2000;
        let start_pos = offset.min(total_length);
        assert_eq!(start_pos, total_length);
        
        // Test when remaining content is less than max_size
        let offset = 980;
        let start_pos = offset.min(total_length);
        let end_pos = (start_pos + max_size).min(total_length);
        let content = &full_text[start_pos..end_pos];
        let has_more = end_pos < total_length;
        
        assert_eq!(start_pos, 980);
        assert_eq!(end_pos, 1000);
        assert_eq!(content.len(), 20);
        assert!(!has_more);
    }
} 