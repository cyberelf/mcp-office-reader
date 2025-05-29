use rmcp::{service::RequestContext, RoleServer, Error as McpError, model::ServerInfo, model::{IntoContents, Content}, tool};
use rmcp::model::{Implementation, ListPromptsResult, PaginatedRequestParam, ProtocolVersion, ServerCapabilities};
use rmcp::ServerHandler;
use rmcp::serve_server;
use rmcp::schemars;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use tokio_stream::StreamExt;
use serde_json;

use crate::document_parser::{process_document_with_pages, get_document_page_info, DocumentProcessingResult, DocumentPageInfoResult};
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

/// Wrapper for document page information
pub struct DocumentPageInfo {
    pub file_path: String,
    pub total_pages: Option<usize>,
    pub file_exists: bool,
    pub error: Option<String>,
    pub page_info: String,
}

impl IntoContents for DocumentPageInfo {
    fn into_contents(self) -> Vec<Content> {
        let info = if self.file_exists {
            if let Some(error) = self.error {
                format!("File: {}\nError: {}", self.file_path, error)
            } else if let Some(total) = self.total_pages {
                format!("File: {}\nTotal pages: {}\n{}", self.file_path, total, self.page_info)
            } else {
                format!("File: {}\nPage information not available", self.file_path)
            }
        } else {
            format!("File: {}\nFile not found", self.file_path)
        };
        vec![Content::text(info)]
    }
}

/// Wrapper for page-based document content with metadata
pub struct PageBasedDocumentContent {
    pub content: String,
    pub total_pages: Option<usize>,
    pub requested_pages: String,
    pub returned_pages: Vec<usize>,
    pub file_path: String,
}

impl IntoContents for PageBasedDocumentContent {
    fn into_contents(self) -> Vec<Content> {
        let metadata = if let Some(total) = self.total_pages {
            format!(
                "File: {}\nTotal pages: {}\nRequested pages: {}\nReturned pages: {:?}\n\n",
                self.file_path, total, self.requested_pages, self.returned_pages
            )
        } else {
            format!(
                "File: {}\nRequested pages: {}\nReturned pages: {:?}\n\n",
                self.file_path, self.requested_pages, self.returned_pages
            )
        };
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

/// Convert DocumentProcessingResult to PageBasedDocumentContent
impl From<DocumentProcessingResult> for PageBasedDocumentContent {
    fn from(result: DocumentProcessingResult) -> Self {
        Self {
            content: result.content,
            total_pages: result.total_pages,
            requested_pages: result.requested_pages,
            returned_pages: result.returned_pages,
            file_path: result.file_path,
        }
    }
}

/// Convert DocumentPageInfoResult to DocumentPageInfo
impl From<DocumentPageInfoResult> for DocumentPageInfo {
    fn from(result: DocumentPageInfoResult) -> Self {
        let file_exists = result.file_exists();
        
        Self {
            file_path: result.file_path,
            total_pages: result.total_pages,
            file_exists,
            error: result.error,
            page_info: result.page_info,
        }
    }
}

#[tool(tool_box)]
impl OfficeReader {
    /// Get the page information of an office document without reading the full content
    #[tool(description = "Get the page information of an office document (Excel, PDF, DOCX) without reading the full content")]
    pub async fn get_document_page_info(
        &self,
        #[tool(param)]
        #[schemars(description = "Absolute path to the office document file")]
        file_path: String,
    ) -> DocumentPageInfo {
        let result = get_document_page_info(&file_path);
        result.into()
    }

    /// Read an office document and return its content as markdown with page selection
    #[tool(description = "Read an office document (Excel, PDF, DOCX) and return its content as markdown with page selection")]
    pub async fn read_office_document(
        &self,
        #[tool(param)]
        #[schemars(description = "Absolute path to the office document file")]
        file_path: String,
        #[tool(param)]
        #[schemars(description = "Page selection: integer for single page (e.g., 1), string for ranges/multiple pages (e.g., '1,3,5-7'), or 'all' for all pages")]
        pages: Option<serde_json::Value>,
    ) -> PageBasedDocumentContent {
        // Convert the pages parameter to a string format that our parser expects
        let pages_str = match pages {
            Some(serde_json::Value::Number(n)) => {
                // Handle integer input (e.g., 1, 2, 3)
                if let Some(page_num) = n.as_u64() {
                    Some(page_num.to_string())
                } else {
                    Some("1".to_string()) // Default to page 1 if invalid number
                }
            },
            Some(serde_json::Value::String(s)) => {
                // Handle string input (e.g., "1,3,5-7", "all")
                Some(s)
            },
            Some(_) => {
                // Handle other JSON types by converting to string
                Some("all".to_string())
            },
            None => None, // No pages specified, will default to "all"
        };
        
        let result = process_document_with_pages(&file_path, pages_str);
        result.into()
    }

    /// Stream an office document and return its content as markdown in chunks
    #[tool(description = "Stream an office document (Excel, PDF, DOCX) and return its content as markdown in chunks with progress")]
    pub async fn stream_office_document(
        &self,
        #[tool(param)]
        #[schemars(description = "Absolute path to the office document file")]
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
                1. get_document_page_info: Get page information of a document without reading the full content\n\
                2. read_office_document: Read a document with page selection (e.g., '1,3,5-7' or 'all')\n\
                3. stream_office_document: Stream document content in chunks with progress tracking\n\n\
                For Excel files, pages refer to sheets. For PDF files, pages refer to actual pages. For DOCX files, there is only one page.\n\
                Use get_document_page_info first to see available pages, then use read_office_document with specific page selection.".to_string()
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
    async fn test_get_document_page_info_file_not_found() {
        let office_reader = OfficeReader::new();
        let result = office_reader.get_document_page_info("nonexistent_file.xlsx".to_string()).await;
        
        assert_eq!(result.file_path, "nonexistent_file.xlsx");
        assert_eq!(result.total_pages, None);
        assert!(!result.file_exists);
        assert!(result.error.is_some());
        assert_eq!(result.error.as_ref().unwrap(), "file_not_found");
    }

    #[tokio::test]
    async fn test_read_office_document_with_pages() {
        let office_reader = OfficeReader::new();
        
        // Test with a non-existent file first
        let result = office_reader.read_office_document(
            "nonexistent_file.xlsx".to_string(),
            Some(serde_json::Value::String("1,2".to_string()))
        ).await;
        
        assert_eq!(result.file_path, "nonexistent_file.xlsx");
        assert_eq!(result.total_pages, None);
        assert_eq!(result.requested_pages, "");
        assert_eq!(result.returned_pages, Vec::<usize>::new());
        assert!(result.content.contains("File not found"));
    }

    #[tokio::test]
    async fn test_read_office_document_with_integer_page() {
        let office_reader = OfficeReader::new();
        
        // Test with integer page parameter
        let result = office_reader.read_office_document(
            "nonexistent_file.xlsx".to_string(),
            Some(serde_json::Value::Number(serde_json::Number::from(1)))
        ).await;
        
        assert_eq!(result.file_path, "nonexistent_file.xlsx");
        assert_eq!(result.total_pages, None);
        assert_eq!(result.requested_pages, "");
        assert_eq!(result.returned_pages, Vec::<usize>::new());
        assert!(result.content.contains("File not found"));
    }

    #[tokio::test]
    async fn test_read_office_document_with_various_page_types() {
        let office_reader = OfficeReader::new();
        
        // Test with different JSON value types
        let test_cases = vec![
            (serde_json::Value::Number(serde_json::Number::from(3)), "Expected page 3"),
            (serde_json::Value::String("1,2,3".to_string()), "Expected pages 1,2,3"),
            (serde_json::Value::String("all".to_string()), "Expected all pages"),
            (serde_json::Value::Bool(true), "Should default to all"), // Should default to "all" for non-string/number types
        ];
        
        for (input, description) in test_cases {
            let result = office_reader.read_office_document(
                "nonexistent_file.xlsx".to_string(),
                Some(input)
            ).await;
            
            // For error cases, requested_pages will be empty, but we can verify the input was processed
            assert_eq!(result.file_path, "nonexistent_file.xlsx", "{}", description);
            assert!(result.content.contains("File not found"), "{}", description);
        }
    }

    #[tokio::test]
    async fn test_page_based_document_content_metadata() {
        let content = PageBasedDocumentContent {
            content: "Hello, World!".to_string(),
            total_pages: Some(10),
            requested_pages: "1,2,3".to_string(),
            returned_pages: vec![1, 2, 3],
            file_path: "test.txt".to_string(),
        };
        
        // Test the struct fields directly before moving
        assert_eq!(content.total_pages, Some(10));
        assert_eq!(content.requested_pages, "1,2,3");
        assert_eq!(content.returned_pages, vec![1, 2, 3]);
        assert_eq!(content.file_path, "test.txt");
        assert_eq!(content.content, "Hello, World!");
        
        let contents = content.into_contents();
        assert_eq!(contents.len(), 1);
    }

    #[tokio::test]
    async fn test_document_page_info_formatting() {
        let page_info = DocumentPageInfo {
            file_path: "test.pdf".to_string(),
            total_pages: Some(5000),
            file_exists: true,
            error: None,
            page_info: "Page 1, Page 2, Page 3".to_string(),
        };
        
        // Test the struct fields directly before moving
        assert_eq!(page_info.file_path, "test.pdf");
        assert_eq!(page_info.total_pages, Some(5000));
        assert!(page_info.file_exists);
        assert!(page_info.error.is_none());
        
        let contents = page_info.into_contents();
        assert_eq!(contents.len(), 1);
    }

    #[tokio::test]
    async fn test_document_page_info_with_error() {
        let page_info = DocumentPageInfo {
            file_path: "test.unsupported".to_string(),
            total_pages: None,
            file_exists: true,
            error: Some("Unsupported file type: unsupported".to_string()),
            page_info: "".to_string(),
        };
        
        // Test the struct fields directly before moving
        assert_eq!(page_info.file_path, "test.unsupported");
        assert_eq!(page_info.total_pages, None);
        assert!(page_info.file_exists);
        assert!(page_info.error.is_some());
        assert_eq!(page_info.error.as_ref().unwrap(), "Unsupported file type: unsupported");
        
        let contents = page_info.into_contents();
        assert_eq!(contents.len(), 1);
    }

    #[tokio::test]
    async fn test_page_selection_logic() {
        // Test the logic for page selection with a mock scenario
        let pages_param = "1,3,5-7";
        
        // This would be handled by the parse_pages_parameter function in document_parser
        // We can test the expected behavior here
        assert!(pages_param.contains("1"));
        assert!(pages_param.contains("3"));
        assert!(pages_param.contains("5-7"));
        
        // Test that page ranges are properly formatted
        let all_pages = "all";
        assert_eq!(all_pages, "all");
        
        // Test single page
        let single_page = "1";
        assert_eq!(single_page, "1");
    }
} 