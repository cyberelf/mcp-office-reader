use rmcp::{service::RequestContext, RoleServer, Error as McpError, model::ServerInfo, model::{IntoContents, Content}, tool};
use rmcp::model::{Implementation, ListPromptsResult, PaginatedRequestParam, ProtocolVersion, ServerCapabilities};
use rmcp::ServerHandler;
use rmcp::serve_server;
use rmcp::schemars;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use tokio_stream::StreamExt;

use crate::document_parser::process_document;
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

#[tool(tool_box)]
impl OfficeReader {
    /// Read an office document and return its content as markdown
    #[tool(description = "Read an office document (Excel, PDF, DOCX) and return its content as markdown")]
    pub async fn read_office_document(
        &self,
        #[tool(param)]
        #[schemars(description = "Path to the office document file")]
        file_path: String,
    ) -> DocumentContent {
        // Process the document
        let markdown = process_document(&file_path);
        DocumentContent { content: markdown }
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
                "This server provides functionality to read and parse office documents (Excel, PDF, DOCX) and return their content as markdown.".to_string()
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