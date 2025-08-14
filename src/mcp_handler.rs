use rmcp::{model::{Content, IntoContents, ServerInfo, CallToolResult, ErrorCode}, service::RequestContext, tool, tool_handler, tool_router, ErrorData, RoleServer};
use rmcp::handler::server::tool::{ToolRouter, Parameters};
use rmcp::model::{Implementation, ListPromptsResult, PaginatedRequestParam, ProtocolVersion, ServerCapabilities};
// Define McpError as an alias for ErrorData as per RMCP 0.5.0
type McpError = ErrorData;
use rmcp::ServerHandler;
use rmcp::serve_server;
use rmcp::schemars;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use tokio_stream::StreamExt;
use serde_json;

use crate::document_parser::{process_document_with_pages, get_document_page_info, DocumentProcessingResult, DocumentPageInfoResult};
use crate::shared_utils::resolve_file_path_string;
use crate::streaming_parser::{stream_pdf_to_markdown, stream_excel_to_markdown, StreamingConfig, ProcessingProgress};
use crate::powerpoint_parser::{
    process_powerpoint_with_slides, 
    get_powerpoint_slide_info, 
    generate_slide_snapshot,
    SlideSnapshotResult,
};

/// Input for the read_office_document tool
#[derive(Serialize, Deserialize, Debug, schemars::JsonSchema)]
pub struct ReadOfficeDocumentInput {
    #[schemars(description = "Path to the office document file")]
    pub file_path: String,
}

/// Input for read by page
#[derive(Serialize, Deserialize, Debug, schemars::JsonSchema)]
pub struct ReadOfficeDocumentByPageInput {
    #[schemars(description = "Path to the office document file")]
    pub file_path: String,
    #[schemars(description = "Page/slide selection: integer for single page (e.g., 1), string for ranges/multiple pages (e.g., '1,3,5-7'), or 'all' for all pages/slides")]
    pub pages: Option<serde_json::Value>,
}

/// Input for read by slide
#[derive(Serialize, Deserialize, Debug, schemars::JsonSchema)]
pub struct ReadOfficeDocumentBySlideInput {
    #[schemars(description = "Path to the office document file")]
    pub file_path: String,
    #[schemars(description = "Slide selection: integer for single slide (e.g., 1), string for ranges/multiple slides (e.g., '1,3,5-7'), or 'all' for all slides")]
    pub slides: Option<serde_json::Value>,
}

/// Input for generate_powerpoint_slide_snapshot
#[derive(Serialize, Deserialize, Debug, schemars::JsonSchema)]
pub struct GeneratePowerpointSlideSnapshotInput {
    #[schemars(description = "Path to the PowerPoint file")]
    pub file_path: String,
    #[schemars(description = "Slide number to capture (1-based index)")]
    pub slide_number: usize,
    #[schemars(description = "Output image format (png, jpg, etc.)")]
    pub output_format: Option<String>,
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

/// Wrapper for PowerPoint slide snapshot
pub struct SlideSnapshot {
    pub slide_number: usize,
    pub image_data: Option<Vec<u8>>,
    pub image_format: String,
    pub error: Option<String>,
}

impl IntoContents for SlideSnapshot {
    fn into_contents(self) -> Vec<Content> {
        if let Some(error) = self.error {
            vec![Content::text(format!("Slide {}: Error - {}", self.slide_number, error))]
        } else if let Some(data) = self.image_data {
            vec![
                Content::text(format!("Slide {} snapshot ({} format, {} bytes)", 
                    self.slide_number, self.image_format, data.len())),
                // Note: In a real implementation, you might want to return the image data
                // as a base64 encoded string or save it to a file and return the path
            ]
        } else {
            vec![Content::text(format!("Slide {}: No image data available", self.slide_number))]
        }
    }
}

/// Convert SlideSnapshotResult to SlideSnapshot
impl From<SlideSnapshotResult> for SlideSnapshot {
    fn from(result: SlideSnapshotResult) -> Self {
        Self {
            slide_number: result.slide_number,
            image_data: result.image_data,
            image_format: result.image_format,
            error: result.error,
        }
    }
}

/// Office document processor struct that implements the MCP tool interface
#[derive(Clone)]
pub struct OfficeReader {
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl OfficeReader {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    /// Get the page information of an office document without reading the full content
    #[tool(description = "Get the page information of an office document (Excel, PDF, DOCX, PowerPoint) without reading the full content")]
    pub async fn get_document_page_info(
        &self,
        params: Parameters<ReadOfficeDocumentInput>,
    ) -> Result<CallToolResult, McpError> {
        // Resolve file path at entry point
        let resolved_path = resolve_file_path_string(&params.0.file_path)
            .map_err(|e| ErrorData::new(ErrorCode::INVALID_PARAMS, e, None))?;
        
        let result = get_document_page_info(&resolved_path);
        let doc_page_info: DocumentPageInfo = result.into();
        Ok(CallToolResult::success(doc_page_info.into_contents()))
    }

    /// Read an office document and return its content as markdown with page selection
    #[tool(description = "Read an office document (Excel, PDF, DOCX, PowerPoint) and return its content as markdown with page/slide selection")]
    pub async fn read_office_document(
        &self,
        params: Parameters<ReadOfficeDocumentByPageInput>,
    ) -> Result<CallToolResult, McpError> {
        log::debug!("🔍 read_office_document: ENTRY POINT - file_path={}, pages={:?}", 
                    params.0.file_path, params.0.pages);
        
        // Resolve file path at entry point
        log::debug!("🔍 read_office_document: Resolving file path: {}", params.0.file_path);
        let resolved_path = match resolve_file_path_string(&params.0.file_path) {
            Ok(path) => {
                log::debug!("🔍 read_office_document: File path resolved successfully: {} -> {}", 
                           params.0.file_path, path);
                path
            },
            Err(e) => {
                log::error!("❌ read_office_document: File path resolution failed: {}", e);
                return Err(ErrorData::new(ErrorCode::INVALID_PARAMS, e, None));
            }
        };
        
        // Convert the pages parameter to a string format that our parser expects
        log::debug!("🔍 read_office_document: Processing pages parameter: {:?}", params.0.pages);
        let pages_str = match params.0.pages {
            Some(serde_json::Value::Number(n)) => {
                // Handle integer input (e.g., 1, 2, 3)
                if let Some(page_num) = n.as_u64() {
                    let page_str = page_num.to_string();
                    log::debug!("🔍 read_office_document: Pages parameter as number: {}", page_str);
                    Some(page_str)
                } else {
                    log::debug!("🔍 read_office_document: Invalid number for pages, defaulting to '1'");
                    Some("1".to_string()) // Default to page 1 if invalid number
                }
            },
            Some(serde_json::Value::String(s)) => {
                // Handle string input (e.g., "1,3,5-7", "all")
                log::debug!("🔍 read_office_document: Pages parameter as string: '{}'", s);
                Some(s)
            },
            Some(_) => {
                // Handle other JSON types by converting to string
                log::debug!("🔍 read_office_document: Pages parameter other type, defaulting to 'all'");
                Some("all".to_string())
            },
            None => {
                log::debug!("🔍 read_office_document: No pages parameter specified, will default to 'all'");
                None // No pages specified, will default to "all"
            }
        };
        
        log::debug!("🔍 read_office_document: About to call process_document_with_pages with resolved_path='{}', pages_str={:?}", 
                   resolved_path, pages_str);
        
        let result = match std::panic::catch_unwind(|| {
            process_document_with_pages(&resolved_path, pages_str)
        }) {
            Ok(result) => {
                log::debug!("🔍 read_office_document: process_document_with_pages completed successfully");
                result
            },
            Err(panic_info) => {
                let panic_msg = if let Some(s) = panic_info.downcast_ref::<String>() {
                    s.clone()
                } else if let Some(s) = panic_info.downcast_ref::<&str>() {
                    s.to_string()
                } else {
                    "Unknown panic occurred".to_string()
                };
                log::error!("❌ read_office_document: PANIC caught in process_document_with_pages: {}", panic_msg);
                return Err(ErrorData::new(ErrorCode::INTERNAL_ERROR, 
                          format!("Internal error during document processing: {}", panic_msg), None));
            }
        };
        
        log::debug!("🔍 read_office_document: Converting result to PageBasedDocumentContent");
        let page_content: PageBasedDocumentContent = result.into();
        
        log::debug!("🔍 read_office_document: SUCCESS - returning content");
        Ok(CallToolResult::success(page_content.into_contents()))
    }

    /// Read a PowerPoint presentation and return its content as markdown with slide selection
    #[tool(description = "Read a PowerPoint presentation (PPT/PPTX) and return its content as markdown with slide selection")]
    pub async fn read_powerpoint_slides(
        &self,
        params: Parameters<ReadOfficeDocumentBySlideInput>,
    ) -> Result<CallToolResult, McpError> {
        // Resolve file path at entry point
        let resolved_path = resolve_file_path_string(&params.0.file_path)
            .map_err(|e| ErrorData::new(ErrorCode::INVALID_PARAMS, e, None))?;
        
        // Convert the slides parameter to a string format
        let slides_str = match params.0.slides {
            Some(serde_json::Value::Number(n)) => {
                if let Some(slide_num) = n.as_u64() {
                    Some(slide_num.to_string())
                } else {
                    Some("1".to_string())
                }
            },
            Some(serde_json::Value::String(s)) => Some(s),
            Some(_) => Some("all".to_string()),
            None => None,
        };
        
        let result = process_powerpoint_with_slides(&resolved_path, slides_str);
        
        // Convert PowerPointProcessingResult to PageBasedDocumentContent
        if let Some(error) = result.error {
            return Err(ErrorData::new(ErrorCode::INVALID_PARAMS, error, None));
        } else {
            let page_content = PageBasedDocumentContent {
                content: result.content,
                total_pages: result.total_slides,
                requested_pages: result.requested_slides,
                returned_pages: result.returned_slides,
                file_path: result.file_path,
            };
            return Ok(CallToolResult::success(page_content.into_contents()));
        };
        
    }

    /// Get PowerPoint slide information without reading the full content
    #[tool(description = "Get PowerPoint slide information (slide count, etc.) without reading the full content")]
    pub async fn get_powerpoint_slide_info(
        &self,
        params: Parameters<ReadOfficeDocumentInput>,
    ) -> Result<CallToolResult, McpError> {
        // Resolve file path at entry point
        let resolved_path = resolve_file_path_string(&params.0.file_path)
            .map_err(|e| ErrorData::new(ErrorCode::INVALID_PARAMS, e, None))?;
        
        let result = get_powerpoint_slide_info(&resolved_path);
        
        // Convert PowerPointPageInfoResult to DocumentPageInfo
        let file_exists = result.file_exists();
        let doc_page_info = DocumentPageInfo {
            file_path: result.file_path,
            total_pages: result.total_slides,
            file_exists,
            error: result.error,
            page_info: result.slide_info,
        };
        
        Ok(CallToolResult::success(doc_page_info.into_contents()))
    }

    /// Generate a snapshot image of a specific PowerPoint slide using native Rust rendering (no external dependencies required)
    #[tool(description = "Generate a snapshot image of a specific PowerPoint slide using native Rust rendering (no external dependencies required)")]
    pub async fn generate_powerpoint_slide_snapshot(
        &self,
        params: Parameters<GeneratePowerpointSlideSnapshotInput>,
    ) -> Result<CallToolResult, McpError> {
        // Resolve file path at entry point
        let resolved_path = resolve_file_path_string(&params.0.file_path)
            .map_err(|e| ErrorData::new(ErrorCode::INVALID_PARAMS, e, None))?;
        
        let format = params.0.output_format.unwrap_or_else(|| "png".to_string());
        let result = generate_slide_snapshot(&resolved_path, params.0.slide_number, &format);
        let slide_snapshot: SlideSnapshot = result.into();
        Ok(CallToolResult::success(slide_snapshot.into_contents()))
    }

    /// Stream an office document and return its content as markdown in chunks
    #[tool(description = "Stream an office document (Excel, PDF, DOCX, PowerPoint) and return its content as markdown in chunks with progress")]
    pub async fn stream_office_document(
        &self,
        params: Parameters<StreamOfficeDocumentInput>,
    ) -> Result<CallToolResult, McpError> {
        use std::path::Path;
        
        // Create streaming config
        let mut config = StreamingConfig::default();
        if let Some(size) = params.0.chunk_size {
            config.max_chunk_size_chars = size;
        }
        
        // Resolve the file path
        let resolved_path = resolve_file_path_string(&params.0.file_path)
            .map_err(|e| ErrorData::new(ErrorCode::INVALID_PARAMS, e, None))?;
        
        // Check if file exists
        if !Path::new(&resolved_path).exists() {
            return Err(ErrorData::new(ErrorCode::INVALID_PARAMS, format!("File not found: {}", resolved_path), None));
        }
        
        // Determine file type from extension
        let extension = Path::new(&resolved_path)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase());
        
        match extension {
            Some(ext) => {
                match ext.as_str() {
                    "pdf" => {
                        // Stream PDF content
                        let mut stream = Box::pin(stream_pdf_to_markdown(&resolved_path, config));
                        let content = if let Some(progress) = stream.next().await {
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
                        };
                        return Ok(CallToolResult::success(content.into_contents()));
                    }
                    "xlsx" | "xls" => {
                        // Stream Excel content
                        let mut stream = Box::pin(stream_excel_to_markdown(&resolved_path, config));
                        let content = if let Some(progress) = stream.next().await {
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
                        };
                        return Ok(CallToolResult::success(content.into_contents()));
                    }
                    _ => {
                        return Err(ErrorData::new(ErrorCode::INVALID_PARAMS, format!("Unsupported file type for streaming: {}", ext), None));
                    }
                }
            }
            None => {
                return Err(ErrorData::new(ErrorCode::INVALID_PARAMS, "Unable to determine file type (no extension)".to_string(), None));
            }
        }
    }
}

#[tool_handler]
impl ServerHandler for OfficeReader {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::default(),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "This server provides functionality to read and parse office documents (Excel, PDF, DOCX, PowerPoint) and return their content as markdown. Available tools:\n\n\
                1. get_document_page_info: Get page information of a document without reading the full content\n\
                2. read_office_document: Read a document with page/slide selection (e.g., '1,3,5-7' or 'all')\n\
                3. read_powerpoint_slides: Read PowerPoint slides with specific slide selection\n\
                4. get_powerpoint_slide_info: Get PowerPoint slide information without reading content\n\
                5. generate_powerpoint_slide_snapshot: Generate image snapshots of PowerPoint slides\n\
                6. stream_office_document: Stream document content in chunks with progress tracking\n\n\
                File Path Support:\n\
                - Supports both absolute and relative file paths\n\
                - Relative paths are resolved using the PROJECT_ROOT environment variable if set\n\
                - Falls back to current working directory if PROJECT_ROOT is not set\n\n\
                For Excel files, pages refer to sheets. For PDF files, pages refer to actual pages. For DOCX files, there is only one page. For PowerPoint files, pages refer to slides.\n\
                Use get_document_page_info or get_powerpoint_slide_info first to see available pages/slides, then use the appropriate read function with specific selection.".to_string()
            ),
        }
    }

    async fn list_prompts(
        &self,
        _request: Option<PaginatedRequestParam>,
        _: RequestContext<RoleServer>,
    ) -> Result<ListPromptsResult, ErrorData> {
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