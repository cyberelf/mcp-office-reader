use rmcp::{service::RequestContext, RoleServer, Error as McpError, model::ServerInfo, model::{IntoContents, Content}, tool};
use rmcp::model::{Implementation, ListPromptsResult, PaginatedRequestParam, ProtocolVersion, ServerCapabilities};
use rmcp::ServerHandler;
use rmcp::serve_server;
use rmcp::schemars;
use serde::{Deserialize, Serialize};
use anyhow::Result;


use crate::document_parser::process_document;

/// Office document processor struct that implements the MCP tool interface
#[derive(Clone)]
pub struct OfficeReader;

impl OfficeReader {
    pub fn new() -> Self {
        OfficeReader
    }
}

/// Input for the process_office_document tool
#[derive(Serialize, Deserialize, Debug, schemars::JsonSchema)]
pub struct ProcessOfficeDocumentInput {
    #[schemars(description = "Path to the office document file")]
    pub file_path: String,
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

#[tool(tool_box)]
impl OfficeReader {
    /// Process an office document and return its content as markdown
    #[tool(description = "Read an office document (Excel, PDF, DOCX) and return its content as markdown")]
    pub async fn process_office_document(
        &self,
        #[tool(param)]
        #[schemars(description = "Path to the office document file")]
        file_path: String,
    ) -> DocumentContent {
        // Process the document
        let markdown = process_document(&file_path);
        DocumentContent { content: markdown }
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