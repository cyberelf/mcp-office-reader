# RMCP (Rust Model Context Protocol)

RMCP is a clean Rust implementation of the Model Context Protocol with tokio async runtime support.

- **Version**: 0.1.5
- **GitHub**: [4t145/rmcp](https://github.com/4t145/rmcp)
- **Documentation**: [docs.rs/rmcp](https://docs.rs/rmcp/latest/rmcp/)

## Key Components

- **Transport Layer**: Handles communication between client and server
- **Service Layer**: Manages the service lifecycle
- **Handler Layer**: Implements the protocol handlers
- **Model**: Defines the data types used in the protocol

## Getting Started

### Installation

Add rmcp to your Cargo.toml:

```toml
[dependencies]
rmcp = { version = "0.1.5", features = ["server"] }  # For server implementation
# OR
rmcp = { version = "0.1.5", features = ["client"] }  # For client implementation
```

### Quick Start Guide

#### Creating a Server

1. Build a transport
2. Create a service handler
3. Serve them together
4. Interact with the server
5. Handle service shutdown

```rust
// 1. Build a transport that implements IntoTransport trait
use tokio::io::{stdin, stdout};
let transport = (stdin(), stdout());

// 2. Create a service handler
let service = YourService::new();

// 3. Serve them together
let server = service.serve(transport).await?;

// 4. Interact with the server
let roots = server.list_roots().await?;

// 5. Handle service shutdown
let quit_reason = server.waiting().await?;
// or cancel it
let quit_reason = server.cancel().await?;
```

## Model Structures

The `rmcp::model` module contains the core data structures of the protocol:

### Message Types

RMCP defines several message types for client-server communication:

- **ClientRequest**: Messages sent from client to server requesting some action
- **ClientNotification**: Notifications sent from client to server (no response expected)
- **ServerRequest**: Messages sent from server to client requesting some action
- **ServerNotification**: Notifications sent from server to client (no response expected)
- **ClientResult**: Responses sent from client to server
- **ServerResult**: Responses sent from server to client

### Tool-Related Structures

Tools are a central concept in RMCP, representing functions that can be called:

```rust
struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: Value, // JSON Schema
    pub return_type: Option<Value>, // JSON Schema
}

struct CallToolRequest {
    // Contains name and parameters for tool execution
    pub name: String,
    pub parameters: Value,
}

struct CallToolResult {
    // Result of tool execution
    pub result: serde_json::Value,
}
```

### Content Types

RMCP supports different types of content that can be exchanged:

```rust
enum RawContent {
    Text(RawTextContent),
    Image(RawImageContent),
    Json(JsonContent),
    Resource(RawResource),
    EmbeddedResource(RawEmbeddedResource),
}

type Content = Annotated<RawContent>;
```

## Transport Options

### Standard I/O Transport

For server-side applications that need to communicate via standard input/output:

```rust
use tokio::io::{stdin, stdout};
let transport = (stdin(), stdout());
```

### Child Process Transport

For client-side applications that need to spawn and communicate with a child process:

```rust
use rmcp::transport::TokioChildProcess;
use std::process::Command;

let transport = TokioChildProcess::new(
    Command::new("npx")
        .arg("-y")
        .arg("@modelcontextprotocol/server-everything")
)?;
```

### Server-Sent Events (SSE) Transport

For web-based applications that need to use Server-Sent Events:

```rust
// Client-side SSE
use rmcp::transport::SseTransport;
let transport = SseTransport::connect("https://example.com/sse-endpoint").await?;

// Server-side SSE
use rmcp::transport::SseServerTransport;
let transport = SseServerTransport::new(request, response);
```

## Tool Macros

RMCP provides macros to easily declare tools:

```rust
use rmcp::{ServerHandler, model::ServerInfo, schemars, tool};

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct MyToolRequest {
    #[schemars(description = "Parameter description")]
    pub param1: i32,
    pub param2: String,
}

#[derive(Debug, Clone)]
pub struct MyToolbox;

// Create a static toolbox to store tool attributes
#[tool(tool_box)]
impl MyToolbox {
    // Async function as a tool
    #[tool(description = "Description of what this tool does")]
    async fn my_tool(&self, #[tool(aggr)] request: MyToolRequest) -> String {
        // Implementation
        format!("Result: {}, {}", request.param1, request.param2)
    }
    
    // Sync function as a tool
    #[tool(description = "Another tool example")]
    fn another_tool(
        &self,
        #[tool(param)]
        #[schemars(description = "First parameter")]
        param1: i32,
        #[tool(param)]
        param2: String,
    ) -> String {
        // Implementation
        format!("Result: {}, {}", param1, param2)
    }
}

// Implement ServerHandler by querying static toolbox
#[tool(tool_box)]
impl ServerHandler for MyToolbox {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("My toolbox description".into()),
            ..Default::default()
        }
    }
}
```

## Example Implementation: Rust Documentation Fetcher

Here's a real-world example implementing a Rust documentation fetcher service using RMCP:

```rust
use rmcp::model::{Implementation, ListPromptsResult, PaginatedRequestParam, ProtocolVersion, ServerCapabilities};
use rmcp::service::RequestContext;
use rmcp::{RoleServer, Error as McpError, ServerHandler, model::ServerInfo, tool};
use rmcp::{schemars, model::{IntoContents, Content}};
use std::sync::Arc;

/// Main struct responsible for fetching and caching Rust documentation.
#[derive(Clone)]
pub struct DocFetcher {
    /// In-memory cache for storing fetched documentation
    cache: Arc<InMemoryCache>,
}

#[tool(tool_box)]
impl DocFetcher {
    /// Creates a new `DocFetcher` instance with the provided cache.
    pub fn new(cache: Arc<InMemoryCache>) -> Self {
        Self { cache }
    }

    /// Fetches documentation for a Rust crate from docs.rs.
    #[tool(description = "Fetch Rust documentation of a specific crate and version")]
    async fn fetch_document(
        &self,
        #[tool(param)]
        #[schemars(description = "Name of the crate to fetch documentation for")]
        crate_name: String,

        #[tool(param)]
        #[schemars(description = "Version of crate, e.g. 1.0.0")]
        version: String,

        #[tool(param)]
        #[schemars(description = "Path to the specific documentation page")]
        path: String,
    ) -> Result<DocContent, DocsFetchError> {
        // Implementation details omitted for brevity
        // ...
    }
}

#[tool(tool_box)]
impl ServerHandler for DocFetcher {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::default(),
            capabilities: ServerCapabilities::builder()
                .enable_tools()  // We only need tools capability
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "This server provides access to Rust documentation from docs.rs.".to_string()
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

// Client usage example
async fn example() {
    let cache = Arc::new(InMemoryCache::new("cache_dir".into()));
    let fetcher = DocFetcher::new(cache);
    
    // Setup transport and serve
    let transport = (stdin(), stdout());
    let server = fetcher.serve(transport).await?;
    
    let quit_reason = server.waiting().await?;
}
```

## Advanced Usage

### Managing Multiple Services

For cases where you need to manage several services:

```rust
let dyn_service = service.into_dyn();
```

### Available Features

- `client`: Client-side SDK
- `server`: Server-side SDK
- `macros`: Default macros

#### Transport Feature Flags

- `transport-io`: Server stdio transport
- `transport-sse-server`: Server SSE transport
- `transport-child-process`: Client stdio transport
- `transport-sse`: Client SSE transport

## Error Handling

RMCP defines an `Error` type alias for error handling:

```rust
pub type Error = ErrorData;

struct ErrorData {
    pub code: ErrorCode,
    pub message: Cow<'static, str>,
    pub data: Option<Value>,
}
```

## Related Resources

- [MCP Specification](https://spec.modelcontextprotocol.io/specification/2024-11-05/)
- [Schema](https://github.com/modelcontextprotocol/specification/blob/main/schema/2024-11-05/schema.ts)

## Revision History

- **2024-05-24**: Initial documentation created based on rmcp 0.1.5 documentation. 
- **2024-06-01**: Added Rust documentation fetcher MCP implementation example. 